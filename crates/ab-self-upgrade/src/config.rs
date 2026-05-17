use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct UpgradeConfig {
    pub api_domain: String,
    pub auto_upgrade_enabled: bool,
    pub auto_upgrade_metadata: String,
    pub auto_upgrade_check_frequency: Duration,
    pub install_dir: PathBuf,
    pub binary_name: String,
    pub current_version: Version,
    pub state_report_url: Option<String>,
}

impl UpgradeConfig {
    pub fn from_str(s: &str) -> Result<Self> {
        let mut api_domain = None;
        let mut auto_upgrade_enabled = None;
        let mut auto_upgrade_metadata = None;
        let mut auto_upgrade_check_frequency = None;

        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "api_domain" => api_domain = Some(value.to_string()),
                    "auto_upgrade_enabled" => auto_upgrade_enabled = Some(value.parse::<bool>().map_err(|_| Error::Layout(format!("invalid bool: {value}")))?),
                    "auto_upgrade_metadata" => auto_upgrade_metadata = Some(value.to_string()),
                    "auto_upgrade_check_frequency" => auto_upgrade_check_frequency = Some(value.parse::<u64>().map_err(|_| Error::Layout(format!("invalid number: {value}")))?),
                    _ => {}
                }
            }
        }

        let api_domain = api_domain.ok_or_else(|| Error::Layout("missing api_domain".into()))?;
        let auto_upgrade_enabled = auto_upgrade_enabled.unwrap_or(true);
        let auto_upgrade_metadata = auto_upgrade_metadata.ok_or_else(|| Error::Layout("missing auto_upgrade_metadata".into()))?;
        let frequency_mins = auto_upgrade_check_frequency.unwrap_or(10);

        Ok(UpgradeConfig {
            api_domain,
            auto_upgrade_enabled,
            auto_upgrade_metadata,
            auto_upgrade_check_frequency: Duration::from_secs(frequency_mins * 60),
            install_dir: PathBuf::from("/var/lib/ab-self-upgrade"),
            binary_name: "app".to_string(),
            current_version: Version::new(0, 1, 0),
            state_report_url: None,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    pub fn with_install_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.install_dir = dir.into();
        self
    }

    pub fn with_binary_name(mut self, name: impl Into<String>) -> Self {
        self.binary_name = name.into();
        self
    }

    pub fn with_current_version(mut self, version: Version) -> Self {
        self.current_version = version;
        self
    }

    pub fn with_state_report_url(mut self, url: impl Into<String>) -> Self {
        self.state_report_url = Some(url.into());
        self
    }

    pub fn metadata_url(&self) -> String {
        format!("{}{}", self.api_domain, self.auto_upgrade_metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let cfg = UpgradeConfig::from_str(
            "api_domain = https://cloud.example.com\n\
             auto_upgrade_enabled = false\n\
             auto_upgrade_metadata = /releases/meta.json\n\
             auto_upgrade_check_frequency = 5\n",
        )
        .unwrap();
        assert_eq!(cfg.api_domain, "https://cloud.example.com");
        assert!(!cfg.auto_upgrade_enabled);
        assert_eq!(cfg.auto_upgrade_metadata, "/releases/meta.json");
        assert_eq!(cfg.auto_upgrade_check_frequency, Duration::from_secs(300));
    }

    #[test]
    fn test_parse_defaults() {
        let cfg = UpgradeConfig::from_str(
            "api_domain = https://example.com\n\
             auto_upgrade_metadata = /meta.json\n",
        )
        .unwrap();
        assert!(cfg.auto_upgrade_enabled);
        assert_eq!(cfg.auto_upgrade_check_frequency, Duration::from_secs(600));
    }

    #[test]
    fn test_parse_missing_required() {
        let result = UpgradeConfig::from_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_url() {
        let cfg = UpgradeConfig::from_str(
            "api_domain = https://example.com\n\
             auto_upgrade_metadata = /v1/meta.json\n",
        )
        .unwrap();
        assert_eq!(cfg.metadata_url(), "https://example.com/v1/meta.json");
    }

    #[test]
    fn test_builder_methods() {
        let cfg = UpgradeConfig::from_str(
            "api_domain = https://example.com\n\
             auto_upgrade_metadata = /meta.json\n",
        )
        .unwrap()
        .with_install_dir("/tmp/test")
        .with_binary_name("myapp")
        .with_current_version(Version::new(2, 0, 0))
        .with_state_report_url("https://example.com/report");
        assert_eq!(cfg.install_dir, PathBuf::from("/tmp/test"));
        assert_eq!(cfg.binary_name, "myapp");
        assert_eq!(cfg.current_version, Version::new(2, 0, 0));
        assert_eq!(cfg.state_report_url.as_deref(), Some("https://example.com/report"));
    }
}
