use anyhow::bail;
use aws_sdk_s3::{Client, operation::list_multipart_uploads::ListMultipartUploadsOutput};
use chrono::{DateTime, Local};
use tabled::{Table, Tabled};

use crate::util::build_table;

#[derive(Tabled)]
struct MultipartUploadInfo {
    num: usize,
    key: String,
    initiated: String,
    timestamp_id: String,
}

/// Lists incomplete multipart uploads in an S3 bucket
/// # Arguments
/// * `client` - A reference to the S3 client
/// * `bucket` - The name of the bucket
/// # Returns
/// * `Result<Table, anyhow::Error>` - `Table` if successful, error if the operation fails
pub async fn list_multipart_uploads(client: &Client, bucket: &str) -> Result<Table, anyhow::Error> {
    let res: ListMultipartUploadsOutput = client
        .list_multipart_uploads()
        .bucket(bucket)
        .send()
        .await?;

    if res.uploads.is_none() {
        bail!("No multipart uploads found in the bucket '{}'", bucket)
    }

    let table: Table = build_table(
        res.uploads.unwrap(),
        |i: usize, o: &aws_sdk_s3::types::MultipartUpload| {
            let timestamp =
                DateTime::from_timestamp_millis(o.initiated().unwrap().to_millis().unwrap())
                    .unwrap()
                    .with_timezone(&Local);

            MultipartUploadInfo {
                num: i + 1,
                key: o.key().unwrap().to_string(),
                initiated: timestamp.format("%b %d, %Y - %H:%M:%S").to_string(),
                timestamp_id: timestamp.timestamp_millis().to_string(),
            }
        },
    );

    Ok(table)
}
