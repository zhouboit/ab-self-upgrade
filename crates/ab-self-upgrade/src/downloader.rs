use std::path::Path;

use futures_util::StreamExt;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::error::Result;

pub async fn download_file(client: &Client, url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let resp = client.get(url).send().await?.error_for_status()?;
    let mut file = File::create(dest).await?;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    Ok(())
}

pub fn extract_checksum(checksum_str: &str) -> Result<String> {
    if let Some(hex) = checksum_str.strip_prefix("sha256:") {
        Ok(hex.to_string())
    } else {
        Ok(checksum_str.to_string())
    }
}
