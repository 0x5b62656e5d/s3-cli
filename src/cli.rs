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
    Init {},
}

#[derive(Subcommand, Debug)]
pub enum BucketCommands {
    List,
    Create {
        #[arg(required = true)]
        name: String,

        #[arg(required = true)]
        region: String,
    },
    Delete {
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum FileCommands {
    List {
        #[arg()]
        bucket: String,
    },
    Delete {
        #[arg(required = true)]
        bucket: String,

        #[arg()]
        key: String,

        #[arg(short, long)]
        force: bool,
    },
    Download {
        #[arg(required = true)]
        bucket: String,

        #[arg()]
        key: String,

        #[arg()]
        location: String,

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
    },
}

#[derive(Subcommand, Debug)]
pub enum MultpartCommands {
    Delete {
        #[arg(required = true)]
        bucket: String,

        #[arg(short, long)]
        all: bool,

        #[arg(
            required_unless_present = "all",
            conflicts_with = "all"
        )]
        key: Option<String>,

        #[arg(
            required_unless_present = "all",
            conflicts_with = "all"
        )]
        timestamp_id: Option<String>,
    },
    List {
        #[arg(required = true)]
        bucket: String,
    },
}
