use aws_sdk_s3::{Client, primitives::ByteStream};
use tokio::{
    fs::{File, create_dir_all},
    io::AsyncWriteExt,
};

/// Downloads a file from an S3 bucket to a specified local directory, with an option to override the filename
/// # Arguments
/// * `client` - A reference to the S3 client
/// * `bucket` - The name of the bucket
/// * `key` - The key (path) of the file to download
/// * `location` - The local directory where the file will be saved
/// * `override_filename` - An optional filename to use instead of the original key name
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub async fn download_file(
    client: &Client,
    bucket: String,
    key: String,
    location: String,
    override_filename: Option<String>,
) -> Result<(), anyhow::Error> {
    let mut out = client
        .get_object()
        .bucket(bucket)
        .key(key.clone())
        .send()
        .await?;

    create_dir_all(location.clone()).await?;

    if let Some(filename) = override_filename {
        write_to_file(&mut out.body, location.clone(), filename).await?;
    } else {
        write_to_file(&mut out.body, location, key).await?;
    }

    Ok(())
}

/// Writes the contents of a ByteStream to a file at the specified location with the given filename
/// # Arguments
/// * `bytestream` - A mutable reference to the ByteStream to write
/// * `location` - The local directory where the file will be saved
/// * `filename` - The name of the file to create
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error
async fn write_to_file(
    bytestream: &mut ByteStream,
    location: String,
    filename: String,
) -> Result<(), anyhow::Error> {
    let mut file = File::create(format!(
        "{}/{}",
        location.replace("\"", "").trim_end_matches('/'),
        filename.replace("\"", "")
    ))
    .await?;

    while let Some(chunk) = bytestream
        .try_next()
        .await
        .map_err(|e| anyhow::anyhow!("S3 stream error: {e:?}"))?
    {
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    Ok(())
}
