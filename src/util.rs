use crate::client::config::{Regions, save_regions};
use aws_sdk_s3::Client;
use tabled::{
    Table, Tabled,
    settings::{Color, Modify, Style, object::Rows},
};

/// Rounds a floating-point number to a specified number of decimal places
/// # Arguments
/// * `value` - The floating-point number to round
/// * `precision` - The number of decimal places to round to
/// # Returns
/// * `f64` - The rounded floating-point number
pub fn round(value: f64, precision: u32) -> f64 {
    let factor: f64 = 10f64.powi(precision as i32);

    (value * factor).round() / factor
}

/// Builds a table from an iterable data source using a given mapping function
/// # Arguments
/// * `data` - An iterable data source
/// * `map_fn` - A function that maps each item in the data source to a type that implements `Tabled`
/// # Returns
/// * `Table` - A formatted table
pub fn build_table<A, B, C, D>(data: A, map_fn: B) -> Table
where
    A: IntoIterator<Item = C>,
    B: Fn(usize, &C) -> D,
    D: Tabled,
{
    Table::new(
        data.into_iter()
            .enumerate()
            .map(|(i, item)| map_fn(i, &item))
            .collect::<Vec<D>>(),
    )
    .with(Style::modern())
    .with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_MAGENTA))
    .to_owned()
}

/// Retrieves the region of an S3 bucket, caching the result in a local configuration
/// # Arguments
/// * `regions` - A mutable reference to the `Regions` struct containing cached bucket regions
/// * `bucket` - The name of the bucket
/// * `client` - A reference to the S3 client
/// * `provider_name` - The name of the provider to use for caching the region
/// # Returns
/// * `Result<String, anyhow::Error>` - The region of the bucket if successful, error if the operation fails
pub async fn get_bucket_region(
    regions: &mut Regions,
    bucket: String,
    client: &Client,
    provider_name: &str,
) -> Result<String, anyhow::Error> {
    if let Some(region) = regions.buckets.get(bucket.as_str()) {
        Ok(region.clone())
    } else {
        let resp = client
            .get_bucket_location()
            .bucket(bucket.clone())
            .send()
            .await?;

        let region: String = match resp.location_constraint() {
            None => "us-east-1".to_string(),
            Some(aws_sdk_s3::types::BucketLocationConstraint::Eu) => "eu-west-1".to_string(),
            Some(other) => other.as_str().to_string(),
        };

        let cloned_region: String = region.clone();

        let region_key: String = format!("{}#{}", provider_name, bucket);

        if !regions.buckets.contains_key(region_key.as_str()) {
            regions.buckets.insert(region_key, cloned_region);

            save_regions(regions)?;
        }

        Ok(region)
    }
}
