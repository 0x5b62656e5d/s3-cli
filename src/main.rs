use crate::{
    buckets::{create::create_bucket, delete::delete_bucket, list_buckets::list_buckets},
    cli::{BucketCommands, Cli, Commands, FileCommands, MultpartCommands, ProviderCommands},
    client::{
        config::{Config, Keys, Regions, get_config, get_regions, init_config},
        init::init_regions,
        s3_client::build_client,
    },
    files::{
        delete::delete_file, download::download_file, list_files::list_files, upload::upload_file,
    },
    misc::provider::{get_provider, set_provider},
    multipart::delete::{delete_all_multipart_uploads, delete_multipart_upload},
    util::get_bucket_region,
};
use anyhow::{Result, bail};
use aws_sdk_s3::Client;
use clap::Parser;
use inquire::Confirm;

mod buckets;
mod cli;
mod client;
mod files;
mod misc;
mod multipart;
mod util;

#[::tokio::main]
async fn main() -> Result<()> {
    init_config()?;

    let mut config: Config = get_config()?;
    let mut regions: Regions = get_regions()?;

    let cloned_config: Config = config.clone();

    let mut provider: &Keys = cloned_config
        .providers
        .get(&config.current_provider)
        .ok_or_else(|| anyhow::anyhow!("Active provider not found in config"))?;

    let mut client: Client = build_client(provider, "us-east-1".to_string()).await?;

    let cli: Cli = Cli::parse();

    match cli.command {
        Commands::Buckets { commands } => match commands {
            BucketCommands::List { custom_provider } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                println!("{}", list_buckets(&client).await?);
            }
            BucketCommands::Create {
                name,
                region,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, region.clone()).await?;
                }

                create_bucket(&client, name.clone(), region).await?;

                println!("Created bucket {:?} successfully", name.clone());
            }
            BucketCommands::Delete {
                name,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, name.clone(), &client).await?,
                )
                .await?;

                match Confirm::new(&format!(
                    "Are you sure you want to delete the bucket {:?}? (y/n)",
                    name.clone()
                ))
                .prompt()
                {
                    Ok(v) => {
                        if !v {
                            bail!("Aborting bucket deletion");
                        }

                        delete_bucket(&client, name.clone()).await?;

                        println!("Deleted bucket {:?} successfully", name.clone());
                    }
                    Err(_) => {
                        bail!("There was an error when confirming bucket deletion");
                    }
                }
            }
        },
        Commands::Files { commands } => match commands {
            FileCommands::List {
                bucket,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                println!("{}", list_files(&client, &bucket).await?);
            }
            FileCommands::Delete {
                bucket,
                key,
                force,
                yes,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                if yes {
                    delete_file(
                        &client,
                        bucket,
                        key.clone(),
                        !provider.endpoint_url.contains("cloudflare"),
                        force,
                    )
                    .await?;

                    println!("Deleted {:?} successfully", key.clone());

                    return Ok(());
                }

                match Confirm::new(&format!(
                    "Are you sure you want to delete the file {:?} from bucket {:?}? (y/n)",
                    key.clone(),
                    bucket.clone()
                ))
                .prompt()
                {
                    Ok(v) => {
                        if !v {
                            bail!("Aborting file deletion");
                        }

                        delete_file(
                            &client,
                            bucket,
                            key.clone(),
                            !provider.endpoint_url.contains("cloudflare"),
                            force,
                        )
                        .await?;

                        println!("Deleted {:?} successfully", key.clone());
                    }
                    Err(_) => {
                        bail!("There was an error when confirming file deletion");
                    }
                }
            }
            FileCommands::Download {
                bucket,
                key,
                location,
                override_filename,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                download_file(&client, bucket, key.clone(), location, override_filename).await?;

                println!("Downloaded {:?} successfully", key.clone());
            }
            FileCommands::Upload {
                bucket,
                location,
                override_filename,
                verbose,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                if let Some(filename) = override_filename {
                    upload_file(&client, bucket, filename, location.clone(), verbose).await?;
                } else {
                    upload_file(
                        &client,
                        bucket,
                        location.clone().split('/').next_back().unwrap().to_string(),
                        location.clone(),
                        verbose,
                    )
                    .await?;
                }

                println!(
                    "Uploaded {:?} successfully",
                    location.split('/').next_back().unwrap().to_string()
                );
            }
        },
        Commands::Multipart { commands } => match commands {
            MultpartCommands::Delete {
                bucket,
                all,
                key,
                timestamp_id,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                if all {
                    delete_all_multipart_uploads(&client, bucket).await?;
                } else {
                    delete_multipart_upload(&client, bucket, key.unwrap(), timestamp_id.unwrap())
                        .await?;
                }
            }
            MultpartCommands::List {
                bucket,
                custom_provider,
            } => {
                if let Some(custom_provider_name) = custom_provider {
                    provider = cloned_config
                        .providers
                        .get(&custom_provider_name)
                        .ok_or_else(|| anyhow::anyhow!("Custom provider not found in config"))?;

                    client = build_client(provider, "us-east-1".to_string()).await?;
                }

                client = build_client(
                    provider,
                    get_bucket_region(&mut regions, bucket.clone(), &client).await?,
                )
                .await?;

                println!(
                    "{}",
                    multipart::list::list_multipart_uploads(&client, &bucket).await?
                );
            }
        },
        Commands::Provider { commands } => match commands {
            ProviderCommands::Get => {
                get_provider(&config)?;
            }
            ProviderCommands::Set { provider_name } => {
                set_provider(&mut config, provider_name)?;
                init_regions(provider).await?;
            }
        },
        Commands::Init {} => {
            init_regions(provider).await?;
        }
    }

    Ok(())
}
