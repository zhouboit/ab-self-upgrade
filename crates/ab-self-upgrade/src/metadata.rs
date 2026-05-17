use std::collections::HashMap;

use reqwest::Client;
use semver::Version;
use serde::Deserialize;

use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct PlatformEntry {
    pub download_url: String,
    pub checksum: String,
}

#[derive(Debug, Deserialize)]
pub struct UpgradeMetadata {
    pub latest_version: Version,
    pub platforms: HashMap<String, PlatformEntry>,
}

impl UpgradeMetadata {
    pub async fn fetch(client: &Client, url: &str) -> Result<Self> {
        let resp = client.get(url).send().await?;
        let meta: Self = resp.json().await?;
        Ok(meta)
    }

    pub fn platform_entry(&self, target: &str) -> Option<&PlatformEntry> {
        self.platforms.get(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata_json() {
        let json = r#"{
            "latest_version": "1.2.3",
            "platforms": {
                "x86_64-apple-darwin": {
                    "download_url": "/releases/v1.2.3/app-x86_64",
                    "checksum": "sha256:abcdef123456"
                }
            }
        }"#;
        let meta: UpgradeMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.latest_version, Version::new(1, 2, 3));
        let entry = meta.platform_entry("x86_64-apple-darwin").unwrap();
        assert_eq!(entry.download_url, "/releases/v1.2.3/app-x86_64");
        assert_eq!(entry.checksum, "sha256:abcdef123456");
    }

    #[test]
    fn test_platform_entry_missing() {
        let meta = UpgradeMetadata {
            latest_version: Version::new(1, 0, 0),
            platforms: HashMap::new(),
        };
        assert!(meta.platform_entry("nonexistent").is_none());
    }
}
