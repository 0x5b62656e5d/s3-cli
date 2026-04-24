use crate::{
    client::{
        config::{self, Keys, save_regions},
        s3_client::build_client,
    },
    util::get_bucket_region,
};

/// Initializes the regions file by fetching the regions of all existing buckets using the default client configuration.
/// # Arguments
/// * `provider_name` - The name of the provider to use for fetching bucket regions
/// * `provider` - A reference to the `Keys` struct containing S3 credentials and
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub async fn init_regions(provider_name: &str, provider: &Keys) -> Result<(), anyhow::Error> {
    let mut regions: config::Regions = config::get_regions()?;

    let default_client: aws_sdk_s3::Client =
        build_client(provider, "us-east-1".to_string()).await?;

    let buckets = default_client.list_buckets().send().await?;
    for b in buckets.buckets().iter() {
        let Some(name) = b.name() else {
            continue;
        };

        if regions
            .buckets
            .contains_key(format!("{}#{}", provider_name, name).as_str())
        {
            continue;
        }

        let region: String = get_bucket_region(
            &mut regions,
            name.to_owned(),
            &default_client,
            provider_name,
        )
        .await?;

        regions
            .buckets
            .insert(format!("{}#{}", provider_name, name), region);
    }

    save_regions(&regions)?;

    Ok(())
}
