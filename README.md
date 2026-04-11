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

> [!NOTE]
> If AWS S3 is being used, make sure to leave `endpoint_url` blank. `endpoint_url` is only required for non-AWS S3 users.

## Usage
```
NAME
    s3-cli - Manages S3 buckets and files

COMMANDS
    init
        Initilizes and builds a local map of buckets and their regions for faster access

    buckets list
        Lists buckets associated with the S3 account
    buckets create <bucket_name> <bucket_region>
        Creates a bucket
    buckets delete <bucket_name>
        Deletes a bucket

    files list <bucket_name>
        Lists all the files in a bucket
    files delete <bucket_name> <file_name> [-f | --force]
        Deletes a file in a bucket
        Apply the `-f` or `--force` flag to delete all versions of a file (Not supported for R2)
    files download <bucket_name> <file_key> <download_location> [-o <override_downloaded_filename>]
        Downloads a file from a bucket to a given location (optionally rename it)
    files upload <bucket_name> <file_location> [-o <override_uploaded_filename>]
        Uploads a file into a bucket (optionally rename it)

    multipart list <bucket_name>
        Lists all multipart uploads in a bucket
    multipart delete <bucket_name> <file_key> <timestamp_id>
        Deletes a multipart upload in a bucket
```