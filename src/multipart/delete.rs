use aws_sdk_s3::{Client, operation::list_multipart_uploads::ListMultipartUploadsOutput};
use chrono::{DateTime, Local};

/// Deletes an incomplete multipart upload from an S3 bucket
/// # Arguments
/// * `client` - A reference to the S3 client
/// * `bucket` - The name of the bucket
/// * `key` - The key (path) of the multipart upload to delete
/// * `timestamp_id` - The timestamp ID of the multipart upload to delete
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub async fn delete_multipart_upload(
    client: &Client,
    bucket: String,
    key: String,
    timestamp_id: String,
) -> Result<(), anyhow::Error> {
    let res: ListMultipartUploadsOutput = client
        .list_multipart_uploads()
        .bucket(&bucket)
        .send()
        .await?;

    if res.uploads.is_none() {
        return Ok(());
    }

    let upload_id = res.uploads.unwrap().iter().filter(|u: &&aws_sdk_s3::types::MultipartUpload| {
        let timestamp =
                DateTime::from_timestamp_millis(u.initiated().unwrap().to_millis().unwrap())
                    .unwrap()
                    .with_timezone(&Local).timestamp_millis();

        timestamp == timestamp_id.parse::<i64>().unwrap()
    }).next().map(|u| u.upload_id.clone());

    if upload_id.is_none() {
        return Ok(());
    }

    if upload_id.as_ref().unwrap().is_none() {
        return Ok(());
    }
    
    client.abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id.unwrap().unwrap())
        .send()
        .await?;
    
    Ok(())
}
