use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::PathBuf};
use toml;

#[derive(Debug, Deserialize, Clone)]
/// Configuration structure for S3 CLI
pub struct Config {
    pub current_provider: String,

    #[serde(flatten)]
    pub providers: HashMap<String, Keys>,
}

#[derive(Debug, Deserialize, Clone)]
/// Keys structure containing S3 credentials and URL endpoint
pub struct Keys {
    pub key_id: String,
    pub secret_key: String,
    pub endpoint_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
/// Regions structure that maps bucket names to their respective regions
pub struct Regions {
    #[serde(default)]
    pub buckets: HashMap<String, String>,
}

/// Loads the config from the config file
/// # Returns
/// * `Result<Config, anyhow::Error>` - `Config` or an error if the operation fails
pub fn get_config() -> Result<Config, anyhow::Error> {
    Ok(toml::de::from_str(&fs::read_to_string(
        get_config_dir().join("config.toml"),
    )?)?)
}

/// Loads the regions from the regions file
/// # Returns
/// * `Result<Regions, anyhow::Error>` - `Regions` or an error if the operation fails
pub fn get_regions() -> Result<Regions, anyhow::Error> {
    Ok(toml::de::from_str(&fs::read_to_string(
        get_config_dir().join("regions.toml"),
    )?)?)
}

/// Gets the configuration directory path
/// # Returns
/// * `PathBuf` - The path to the configuration directory
pub fn get_config_dir() -> PathBuf {
    env::home_dir().unwrap().join(".config/s3-cli")
}

/// Initializes the configuration directory and files if they do not exist
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub fn init_config() -> Result<(), anyhow::Error> {
    let config_dir: PathBuf = get_config_dir();

    if !config_dir.is_dir() {
        fs::create_dir_all(&config_dir)?;
    }

    if let Ok(res) = fs::exists(config_dir.join("config.toml"))
        && !res
    {
        fs::write(
            config_dir.join("config.toml"),
            r#"current_provider = "default"

[default]
key_id = ""
secret_key = ""
endpoint_url = ""
"#,
        )?;
    }

    if let Ok(res) = fs::exists(config_dir.join("regions.toml"))
        && !res
    {
        fs::write(config_dir.join("regions.toml"), "[buckets]\n")?;
    }

    Ok(())
}

/// Saves the regions to the regions file
/// # Arguments
/// * `regions` - A reference to the `Regions` structure to be saved
/// # Returns
/// * `Result<(), anyhow::Error>` - `Ok(())` if successful, error if the operation fails
pub fn save_regions(regions: &Regions) -> Result<(), anyhow::Error> {
    let path: PathBuf = get_config_dir().join("regions.toml");
    fs::write(path, toml::to_string(&regions).unwrap())?;

    Ok(())
}
