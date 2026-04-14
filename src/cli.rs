use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Buckets {
        #[command(subcommand)]
        commands: BucketCommands,
    },
    Files {
        #[command(subcommand)]
        commands: FileCommands,
    },
    Multipart {
        #[command(subcommand)]
        commands: MultpartCommands,
    },
    Provider {
        #[command(subcommand)]
        commands: ProviderCommands,
    },
    Init {},
}

#[derive(Subcommand, Debug)]
pub enum BucketCommands {
    List {
        #[arg(short, long)]
        custom_provider: Option<String>,
    },
    Create {
        #[arg(required = true)]
        name: String,

        #[arg(required = true)]
        region: String,

        #[arg(short, long)]
        custom_provider: Option<String>,
    },
    Delete {
        #[arg(required = true)]
        name: String,

        #[arg(short, long)]
        custom_provider: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum FileCommands {
    List {
        #[arg()]
        bucket: String,

        #[arg(short, long)]
        custom_provider: Option<String>,
    },
    Delete {
        #[arg(required = true)]
        bucket: String,

        #[arg()]
        key: String,

        #[arg(short, long)]
        custom_provider: Option<String>,

        #[arg(short, long)]
        force: bool,

        #[arg(short, long)]
        yes: bool,
    },
    Download {
        #[arg(required = true)]
        bucket: String,

        #[arg()]
        key: String,

        #[arg()]
        location: String,

        #[arg(short, long)]
        custom_provider: Option<String>,

        #[arg(short, long)]
        override_filename: Option<String>,
    },
    Upload {
        #[arg(required = true)]
        bucket: String,

        #[arg()]
        location: String,

        #[arg(short, long)]
        override_filename: Option<String>,

        #[arg(short, long)]
        custom_provider: Option<String>,

        #[arg(short, long, default_value = "false")]
        verbose: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum MultpartCommands {
    Delete {
        #[arg(required = true)]
        bucket: String,

        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        custom_provider: Option<String>,

        #[arg(required_unless_present = "all", conflicts_with = "all")]
        key: Option<String>,

        #[arg(required_unless_present = "all", conflicts_with = "all")]
        timestamp_id: Option<String>,
    },
    List {
        #[arg(required = true)]
        bucket: String,

        #[arg(short, long)]
        custom_provider: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProviderCommands {
    Get,
    Set {
        #[arg(required = true)]
        provider_name: String,
    },
}
