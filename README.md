# s3-cli

A simple CLI tool to manage S3 buckets and files from the terminal

## Setup

### Install

#### Prerequisites

- Cargo

<br>

Run `cargo install pepper-s3-cli`

OR

1. Clone this repository
    a. `git clone https://github.com/0x5b62656e5d/s3-cli` or `gh repo clone 0x5b62656e5d/s3-cli`
2. Install the binary to path
    a. `cargo install --path .`

### Environment variables setup

This tool requires API keys acquired from a S3 service. API keys must be granted `Admin Read & Write` in order for bucket and file management to work properly.

Create a `config.toml` file under `~/.config/s3-cli/`

Configuration template:
```toml
[default]
key_id = "Key ID"
secret_key = "Secret Key"
endpoint_url = "Endpoint URL"
```

Users who wish to save different S3-compatible configurations can do so by adding a new record
in `config.toml`. For example:

```toml
[default]
key_id = "Key ID"
secret_key = "Secret Key"
endpoint_url = "Endpoint URL"

[R2]
key_id = "Key ID"
secret_key = "Secret Key"
endpoint_url = "Endpoint URL"
```

> [!NOTE]
> If AWS S3 is being used, make sure to leave `endpoint_url` blank. `endpoint_url` is only required for non-AWS S3 users.

## Usage
```
NAME
    s3-cli - Manages S3 buckets and files

COMMANDS
    init
        Initilizes and builds a local map of buckets and their regions for faster access

    buckets list [-c | --custom-provider]
        Lists buckets associated with the S3 account
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    buckets create <bucket_name> <bucket_region> [-c | --custom-provider]
        Creates a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    buckets delete <bucket_name> [-c | --custom-provider]
        Deletes a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use

    files list <bucket_name> [-c | --custom-provider]
        Lists all the files in a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    files delete <bucket_name> <file_name> [-f | --force] [-y | --yes] [-c | --custom-provider]
        Deletes a file in a bucket
        Apply the `-f` or `--force` flag to delete all versions of a file (Not supported for R2)
        Apply the `-y` or `--yes` flag to bypass confirmation
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    files download <bucket_name> <file_key> <download_location> [-o <override_downloaded_filename>] [-c | --custom-provider]
        Downloads a file from a bucket to a given location (optionally rename it)
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    files upload <bucket_name> <file_location> [-o <override_uploaded_filename>] [-c | --custom-provider] [-v | --verbose]
        Uploads a file into a bucket (optionally rename it)
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
        Apply the `-v` or `--verbose` flag for verbose output

    multipart list <bucket_name> [-c | --custom-provider]
        Lists all multipart uploads in a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    multipart delete <bucket_name> <file_key> <timestamp_id> [-c | --custom-provider]
        Deletes a multipart upload in a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use
    multipart delete <bucket_name> <-a | --all> [-c | --custom-provider]
        Deletes all multipart upload in a bucket
        Use the `-c` or `--custom-provider` flag to specify which S3-compatible provider to use

    provider set <provider_name>
        Sets the desired S3-compatible provider based on config.toml
    provider get
        Gets all the S3-compatible providers configured in config.toml
```