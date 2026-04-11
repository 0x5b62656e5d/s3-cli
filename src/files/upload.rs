use anyhow::bail;
use aws_sdk_s3::{
    Client,
    operation::{
        create_multipart_upload::CreateMultipartUploadOutput, upload_part::UploadPartOutput,
    },
    primitives::{ByteStream, Length},
    types::{CompletedMultipartUpload, CompletedPart},
};
use std::{
    fs,
    io::{Write, stdout},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, MutexGuard},
    task::JoinSet,
    time::{Interval, interval},
};
use tree_magic::from_u8;

const CHUNK_SIZE: u64 = 1024 * 1024 * 8; // 8 MB
const MAX_CHUNKS: u64 = 10000;
const MAX_CONCURRENCY: usize = 32;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct SharedState {
    uploaded_bytes_total: AtomicU64,
    uploaded_bytes_window: AtomicU64,
    active_uploads: AtomicUsize,
    next_chunk_idx: AtomicU64,
    target_concurrency: AtomicUsize,
    stop_flag: AtomicBool,
    uploaded_parts: Mutex<Vec<CompletedPart>>,
    client: Client,
    bucket: String,
    key: String,
    upload_id: String,
    path: PathBuf,
    upload_speed_kb: AtomicU64,
}

/// Uploads a file to an S3 bucket at the specified key (path).
/// # Arguments
/// * `client` - A reference to the S3 client
/// * `bucket` - The name of the bucket
/// * `key` - The key (path) where the file will be uploaded
/// * `file_path` - The local path of the file to upload
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub async fn upload_file(
    client: &Client,
    bucket: String,
    key: String,
    file_path: String,
    verbose: bool,
) -> Result<(), anyhow::Error> {
    let path: &Path = Path::new(&file_path);
    let start: Instant = Instant::now();

    let file_size: u64 = tokio::fs::metadata(path)
        .await
        .expect("Failed to get file metadata")
        .len();

    if verbose {
        println!("File size: {} bytes", file_size);
    }

    if file_size == 0 {
        bail!("Bad file size.");
    }

    if file_size <= 50 * 1024 * 1024 {
        let bytes: ByteStream = ByteStream::from(fs::read(file_path)?);

        client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type(from_u8(bytes.bytes().unwrap()))
            .body(bytes)
            .send()
            .await?;

        println!("Upload completed in {} seconds", start.elapsed().as_secs());

        return Ok(());
    }

    let mut chunk_count: u64 = (file_size / CHUNK_SIZE) + 1;
    let mut prev_chunk_size: u64 = file_size % CHUNK_SIZE;
    if prev_chunk_size == 0 {
        prev_chunk_size = CHUNK_SIZE;
        chunk_count -= 1;
    }

    if verbose {
        println!("Total chunks: {}", chunk_count);
    }

    if chunk_count > MAX_CHUNKS {
        bail!(
            "File is too large to upload. Maximum number of chunks is {}",
            MAX_CHUNKS
        );
    }

    let multipart_upload_res: CreateMultipartUploadOutput = client
        .create_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await?;

    let upload_id = multipart_upload_res.upload_id().ok_or_else(|| {
        anyhow::anyhow!("Failed to initiate multipart upload: No upload ID returned")
    })?;

    let state: Arc<SharedState> = Arc::new(SharedState {
        uploaded_bytes_total: AtomicU64::new(0),
        uploaded_bytes_window: AtomicU64::new(0),
        active_uploads: AtomicUsize::new(0),
        next_chunk_idx: AtomicU64::new(0),
        target_concurrency: AtomicUsize::new(4),
        stop_flag: AtomicBool::new(false),
        uploaded_parts: Mutex::new(Vec::new()),
        client: client.clone(),
        bucket: bucket.to_string(),
        key: key.to_string(),
        upload_id: upload_id.to_string(),
        path: path.to_path_buf(),
        upload_speed_kb: AtomicU64::new(0),
    });

    let file_size_spinner_task: u64 = file_size;
    let state_spinner_task: Arc<SharedState> = Arc::clone(&state);

    let spinner_task: tokio::task::JoinHandle<()> =
        spawn_progress_task(state_spinner_task, file_size_spinner_task, verbose);

    let controller_handle: tokio::task::JoinHandle<()> =
        spawn_controller(state.clone(), MAX_CONCURRENCY, file_size);

    let scheduler_result: Result<(), anyhow::Error> =
        run_scheduler(chunk_count, prev_chunk_size, state.clone()).await;
    state.stop_flag.store(true, Ordering::Relaxed);
    let _ = spinner_task.await;
    let _ = controller_handle.await;

    println!();

    if let Err(e) = scheduler_result {
        let _ = client
            .abort_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(upload_id)
            .send()
            .await;

        bail!("Upload failed: {}", e);
    }

    let mut parts: tokio::sync::MutexGuard<'_, Vec<CompletedPart>> =
        state.uploaded_parts.lock().await;
    parts.sort_by_key(|part| part.part_number);

    if parts.len() as u64 != chunk_count {
        let _ = client
            .abort_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(upload_id)
            .send()
            .await;

        bail!(
            "Upload failed: Expected {} parts but got {}",
            chunk_count,
            parts.len()
        );
    }

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(parts.clone()))
        .build();

    let _ = client
        .complete_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;

    println!("Upload completed in {} seconds", start.elapsed().as_secs());

    Ok(())
}

fn spawn_controller(
    state: Arc<SharedState>,
    max_concurrency: usize,
    file_size: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker: Interval = interval(Duration::from_secs(2));
        let mut smoothed_us_mb_s: f64 = 0.0;
        let mut prev_diff: i8 = 1;
        let mut baseline_throughput: Option<f64> = None;
        let mut cooldown: usize = 0;
        let mut current_target: usize = state.target_concurrency.load(Ordering::Relaxed);
        let mut best_target: usize = current_target;
        let mut best_throughput: f64 = 0.0;
        let mut prev_target: usize = current_target;
        let mut stable_windows: usize = 0;

        loop {
            if state.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            ticker.tick().await;

            if cooldown > 0 {
                cooldown -= 1;
                continue;
            }

            if state.uploaded_bytes_total.load(Ordering::Relaxed) * 100 / file_size >= 80 {
                current_target = best_target;
                state
                    .target_concurrency
                    .store(current_target, Ordering::Relaxed);
                continue;
            }

            let window_bytes: u64 = state.uploaded_bytes_window.swap(0, Ordering::Relaxed);

            let instant_us_mb_s: f64 = (window_bytes as f64) / (2.0 * 1024.0 * 1024.0);

            if instant_us_mb_s > 0.0 {
                if smoothed_us_mb_s == 0.0 {
                    smoothed_us_mb_s = instant_us_mb_s;
                } else {
                    smoothed_us_mb_s = (0.7 * smoothed_us_mb_s) + (0.3 * instant_us_mb_s);
                }
            }

            state
                .upload_speed_kb
                .store((smoothed_us_mb_s * 1024.0) as u64, Ordering::Relaxed);

            if current_target == prev_target {
                stable_windows += 1;
            } else {
                stable_windows = 0;
                prev_target = current_target;
            }

            if state.active_uploads.load(Ordering::Relaxed) >= current_target.saturating_sub(1)
                && current_target == prev_target
                && stable_windows >= 2
                && smoothed_us_mb_s > best_throughput * 1.03
            {
                best_throughput = smoothed_us_mb_s;
                best_target = current_target;
            }

            if baseline_throughput.is_none() {
                baseline_throughput = Some(smoothed_us_mb_s);
                current_target = (current_target + 1).min(max_concurrency);
                state
                    .target_concurrency
                    .store(current_target, Ordering::Relaxed);
                cooldown = 1;
                continue;
            }

            let prev: f64 = baseline_throughput.unwrap();

            let rel_diff: f64 = if prev > 0.0 {
                (smoothed_us_mb_s - prev) / prev
            } else {
                0.0
            };

            if rel_diff >= 0.05 {
                current_target = if prev_diff > 0 {
                    (current_target + 1).min(max_concurrency)
                } else {
                    current_target.saturating_sub(1).max(2)
                };

                baseline_throughput = Some(smoothed_us_mb_s);
                cooldown = 1;
            } else if rel_diff <= -0.2 {
                if smoothed_us_mb_s < best_throughput * 0.90 {
                    current_target = best_target;
                    baseline_throughput = Some(best_throughput);
                    cooldown = 1;
                    prev_diff = 1; // or keep previous direction, depending on how you want probing to resume
                } else {
                    prev_diff = -prev_diff;

                    current_target = if prev_diff > 0 {
                        (current_target + 1).min(max_concurrency)
                    } else {
                        current_target.saturating_sub(1).max(2)
                    };

                    baseline_throughput = Some(smoothed_us_mb_s);
                    cooldown = 1;
                }
            } else {
                baseline_throughput = Some(smoothed_us_mb_s);
            }

            state
                .target_concurrency
                .store(current_target, Ordering::Relaxed);
        }
    })
}

async fn run_scheduler(
    chunk_count: u64,
    prev_chunk_size: u64,
    state: Arc<SharedState>,
) -> Result<(), anyhow::Error> {
    let mut join_set: JoinSet<Result<(CompletedPart, u64), anyhow::Error>> = JoinSet::new();
    let mut scheduler_error: Option<anyhow::Error> = None;
    let mut aborting: bool = false;

    loop {
        while !aborting
            && state.active_uploads.load(Ordering::Relaxed)
                < state.target_concurrency.load(Ordering::Relaxed)
            && state.next_chunk_idx.load(Ordering::Relaxed) < chunk_count
        {
            let chunk_idx: u64 = state.next_chunk_idx.fetch_add(1, Ordering::Relaxed);
            let chunk_size: u64 = chunk_size_for(chunk_idx, chunk_count, prev_chunk_size);

            state.active_uploads.fetch_add(1, Ordering::Relaxed);

            let state_clone = Arc::clone(&state);

            join_set.spawn(async move {
                let part_number = (chunk_idx as i32) + 1;

                let completed_part =
                    upload_part(state_clone, part_number, chunk_idx, chunk_size).await?;

                Ok((completed_part, chunk_size))
            });
        }

        if state.next_chunk_idx.load(Ordering::Relaxed) >= chunk_count
            && state.active_uploads.load(Ordering::Relaxed) == 0
        {
            break;
        }

        if let Some(result) = join_set.join_next().await {
            state.active_uploads.fetch_sub(1, Ordering::Relaxed);

            match result {
                Ok(Ok((completed_part, bytes_uploaded))) => {
                    if !aborting {
                        {
                            let mut parts: MutexGuard<'_, Vec<CompletedPart>> =
                                state.uploaded_parts.lock().await;
                            parts.push(completed_part);
                        }
                        state
                            .uploaded_bytes_total
                            .fetch_add(bytes_uploaded, Ordering::Relaxed);
                        state
                            .uploaded_bytes_window
                            .fetch_add(bytes_uploaded, Ordering::Relaxed);
                    }
                }
                Ok(Err(e)) => {
                    if !aborting {
                        aborting = true;
                        scheduler_error = Some(anyhow::anyhow!("Upload part failed: {}", e));
                        join_set.abort_all();
                    }
                }
                Err(join_error) => {
                    if !aborting {
                        aborting = true;
                        scheduler_error = Some(anyhow::anyhow!("Task join error: {}", join_error));
                        join_set.abort_all();
                    }
                }
            }
        }
    }

    state.active_uploads.store(0, Ordering::Relaxed);

    if let Some(error) = scheduler_error {
        bail!("Upload failed: {}", error);
    }

    Ok(())
}

async fn upload_part(
    state: Arc<SharedState>,
    part_number: i32,
    chunk_idx: u64,
    chunk_size: u64,
) -> Result<CompletedPart, anyhow::Error> {
    let stream: ByteStream = ByteStream::read_from()
        .path(state.path.clone())
        .offset(chunk_idx * CHUNK_SIZE)
        .length(Length::Exact(chunk_size))
        .build()
        .await?;

    let upload_part_res: UploadPartOutput = state
        .client
        .upload_part()
        .key(state.key.clone())
        .bucket(state.bucket.clone())
        .upload_id(state.upload_id.clone())
        .body(stream)
        .part_number(part_number)
        .send()
        .await?;

    Ok(CompletedPart::builder()
        .e_tag(upload_part_res.e_tag.unwrap_or_default())
        .part_number(part_number)
        .build())
}

fn spawn_progress_task(
    state_spinner_task: Arc<SharedState>,
    file_size_spinner_task: u64,
    verbose: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker: Interval = interval(Duration::from_millis(100));
        let mut frame_idx: usize = 0;

        loop {
            ticker.tick().await;

            if verbose {
                print!(
                    "\r{}% {} - Concurrent uploads: {} - Target concurrency: {} - Upload speed: {:.2} MB/s",
                    state_spinner_task
                        .uploaded_bytes_total
                        .load(Ordering::Relaxed)
                        * 100
                        / file_size_spinner_task,
                    SPINNER_FRAMES[frame_idx],
                    state_spinner_task.active_uploads.load(Ordering::Relaxed),
                    state_spinner_task
                        .target_concurrency
                        .load(Ordering::Relaxed),
                    state_spinner_task.upload_speed_kb.load(Ordering::Relaxed) as f64 / 1024.0
                );
            } else {
                print!(
                    "\r{}% {}",
                    state_spinner_task
                        .uploaded_bytes_total
                        .load(Ordering::Relaxed)
                        * 100
                        / file_size_spinner_task,
                    SPINNER_FRAMES[frame_idx],
                );
            }

            if let Err(e) = stdout().flush() {
                eprintln!("Failed to flush stdout: {}", e);
            }

            frame_idx += 1;

            if frame_idx >= SPINNER_FRAMES.len() {
                frame_idx = 0;
            }

            if state_spinner_task.stop_flag.load(Ordering::Relaxed) {
                break;
            }
        }
    })
}

fn chunk_size_for(chunk_index: u64, chunk_count: u64, prev_chunk_size: u64) -> u64 {
    if chunk_index == chunk_count - 1 {
        prev_chunk_size
    } else {
        CHUNK_SIZE
    }
}
