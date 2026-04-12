use crate::{
    client::{
        config::{self, Keys, save_regions},
        s3_client::build_client,
    },
    util::get_bucket_region,
};

/// Initializes the regions file by fetching the regions of all existing buckets using the default client configuration.
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub async fn init_regions(provider: &Keys) -> Result<(), anyhow::Error> {
    let mut regions: config::Regions = config::get_regions()?;

    let default_client: aws_sdk_s3::Client =
        build_client(provider, "us-east-1".to_string()).await?;

    let buckets = default_client.list_buckets().send().await?;
    for b in buckets.buckets().iter() {
        let Some(name) = b.name() else {
            continue;
        };

        if regions.buckets.contains_key(name) {
            continue;
        }

        let region: String =
            get_bucket_region(&mut regions, name.to_string(), &default_client).await?;

        regions.buckets.insert(name.to_string(), region);
    }

    save_regions(&regions)?;

    Ok(())
}
