use anyhow::bail;
use aws_sdk_s3::{
    Client,
    operation::create_multipart_upload::CreateMultipartUploadOutput,
    primitives::{ByteStream, Length},
    types::{CompletedMultipartUpload, CompletedPart},
};
use std::{
    fs,
    io::{Write, stdout},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::time::interval;
use tree_magic::from_u8;

const CHUNK_SIZE: u64 = 1024 * 1024 * 10; // 10 MB
const MAX_CHUNKS: u64 = 10000;
const SPINNER_FRAMES: &'static [&'static str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
) -> Result<(), anyhow::Error> {
    let path: &Path = Path::new(&file_path);

    let file_size = tokio::fs::metadata(path)
        .await
        .expect("Failed to get file metadata")
        .len();

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

        return Ok(());
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

    let mut chunk_count = (file_size / CHUNK_SIZE) + 1;
    let mut size_of_last_chunk = file_size % CHUNK_SIZE;
    if size_of_last_chunk == 0 {
        size_of_last_chunk = CHUNK_SIZE;
        chunk_count -= 1;
    }

    if chunk_count > MAX_CHUNKS {
        bail!(
            "File is too large to upload. Maximum number of chunks is {}",
            MAX_CHUNKS
        );
    }

    let mut upload_parts: Vec<aws_sdk_s3::types::CompletedPart> = Vec::new();

    let is_uploading = Arc::new(AtomicBool::new(true));
    let is_uploading_task = Arc::clone(&is_uploading);

    let spinner_progress = Arc::new(AtomicU64::new(0));
    let spinner_progress_task = Arc::clone(&spinner_progress);

    let task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(100));
        let mut frame_idx: usize = 0;

        loop {
            ticker.tick().await;
            let progress = spinner_progress_task.load(Ordering::Relaxed);

            print!(
                "\r{}% {}",
                (progress) * 100 / chunk_count,
                SPINNER_FRAMES[frame_idx]
            );
            stdout().flush().unwrap();

            frame_idx += 1;

            if frame_idx >= SPINNER_FRAMES.len() {
                frame_idx = 0;
            }

            if chunk_count == progress || !is_uploading_task.load(Ordering::Relaxed) {
                break;
            }
        }
    });

    let upload_res: Result<(), anyhow::Error> = async {
        for chunk_index in 0..chunk_count {
            let this_chunk = if chunk_count - 1 == chunk_index {
                size_of_last_chunk
            } else {
                CHUNK_SIZE
            };
            let stream = ByteStream::read_from()
                .path(path)
                .offset(chunk_index * CHUNK_SIZE)
                .length(Length::Exact(this_chunk))
                .build()
                .await
                .unwrap();

            let part_number = (chunk_index as i32) + 1;
            let upload_part_res = client
                .upload_part()
                .key(&key)
                .bucket(&bucket)
                .upload_id(upload_id)
                .body(stream)
                .part_number(part_number)
                .send()
                .await?;

            upload_parts.push(
                CompletedPart::builder()
                    .e_tag(upload_part_res.e_tag.unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            );

            spinner_progress.store(chunk_index + 1, Ordering::Relaxed);
        }
        Ok(())
    }
    .await;

    is_uploading.store(false, Ordering::Relaxed);
    let _ = task.await;

    upload_res?;

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(upload_parts))
        .build();

    let _ = client
        .complete_multipart_upload()
        .bucket(&bucket)
        .key(&key)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await?;

    Ok(())
}
