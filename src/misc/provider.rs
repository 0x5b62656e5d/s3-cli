use std::fs;

use anyhow::bail;
use toml_edit::DocumentMut;

use crate::client::config::{Config, get_config_dir};

/// Gets the current active provider and lists all available providers along with their endpoint domains
/// # Arguments
/// * `config` - A reference to the configuration struct containing provider information
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub fn get_provider(config: &Config) -> Result<(), anyhow::Error> {
    println!("Current provider: {:?}", config.current_provider);

    println!("Available providers:");
    for (name, key) in config.providers.iter() {
        let endpoint_split = key.endpoint_url.split('.').collect::<Vec<&str>>();

        let endpoint_domain = endpoint_split
            .iter()
            .skip(endpoint_split.len().saturating_sub(2))
            .copied()
            .collect::<Vec<_>>()
            .join(".");

        println!(
            "- {} (Endpoint domain: {})",
            name,
            if endpoint_domain.is_empty() {
                "Unconfigured".to_string()
            } else {
                endpoint_domain
            }
        );
    }

    Ok(())
}

/// Sets the desired provider for S3 operations
/// # Arguments
/// * `config` - A mutable reference to the configuration struct
/// * `provider_name` - The name of the provider to set as active
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub fn set_provider(config: &mut Config, provider_name: String) -> Result<(), anyhow::Error> {
    if !config.providers.contains_key(&provider_name) {
        bail!("Provider {:?} not found in config", provider_name);
    }

    config.current_provider = provider_name.clone();

    let content = fs::read_to_string(get_config_dir().join("config.toml"))?;
    let mut doc = content.parse::<DocumentMut>()?;
    doc["current_provider"] = provider_name.clone().into();

    fs::write(get_config_dir().join("config.toml"), doc.to_string())?;

    println!("Set active provider to {:?}", provider_name);

    Ok(())
}
