use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use reqwest::Client;
use tokio::time::sleep;

use crate::config::UpgradeConfig;
use crate::downloader::{self, download_file};
use crate::error::{Error, Result};
use crate::layout::{InstallLayout, Slot};
use crate::metadata::UpgradeMetadata;
use crate::reporter::{report_upgrade, UpgradeReport, UpgradeStatus};
use crate::state::{Phase, UpgradeOutcome, UpgradeState};
use crate::verifier::verify_sha256;

pub struct UpgradeEngine {
    pub config: UpgradeConfig,
    pub client: Client,
    pub state: UpgradeState,
    layout: InstallLayout,
}

impl UpgradeEngine {
    pub fn new(config: UpgradeConfig) -> Result<Self> {
        let layout = InstallLayout::new(config.install_dir.clone());
        layout.init_dirs()?;

        let state = if layout.state_file().exists() {
            UpgradeState::load(&layout.state_file())?
        } else {
            let s = UpgradeState::new(&config.current_version.to_string());
            s.save(&layout.state_file())?;
            s
        };

        Ok(Self {
            config,
            client: Client::new(),
            state,
            layout,
        })
    }

    pub fn state(&self) -> &UpgradeState {
        &self.state
    }

    pub async fn check_and_upgrade(&mut self) -> Result<UpgradeOutcome> {
        if !self.config.auto_upgrade_enabled {
            return Ok(UpgradeOutcome::AlreadyUpToDate);
        }

        self.set_phase(Phase::CheckingForUpdate);
        let metadata_url = self.config.metadata_url();
        let metadata = UpgradeMetadata::fetch(&self.client, &metadata_url).await?;

        if metadata.latest_version <= self.config.current_version {
            self.set_phase(Phase::Idle);
            return Err(Error::AlreadyUpToDate {
                current: self.config.current_version.clone(),
                latest: metadata.latest_version,
            });
        }

        let target = self.target_platform();
        let entry = metadata.platform_entry(target).ok_or_else(|| {
            Error::NoPlatformMatch(target.to_string())
        })?;

        let download_url = format!("{}{}", self.config.api_domain, entry.download_url);
        let checksum = downloader::extract_checksum(&entry.checksum)?;

        self.state.target_version = Some(metadata.latest_version.to_string());
        self.save_state()?;

        // Download
        self.set_phase(Phase::Downloading);
        let download_path = self.layout.download_dir().join(format!(
            "{}-{}",
            metadata.latest_version,
            self.config.binary_name
        ));
        self.state.downloaded_file = Some(download_path.to_string_lossy().to_string());
        self.save_state()?;

        download_file(&self.client, &download_url, &download_path).await.map_err(|e| {
            Error::DownloadFailed {
                url: download_url.clone(),
                reason: e.to_string(),
            }
        })?;

        // Verify
        self.set_phase(Phase::Verifying);
        verify_sha256(&download_path, &checksum)?;
        tracing::info!(version = %metadata.latest_version, "checksum verified");

        // Install to inactive slot
        self.set_phase(Phase::Installing);
        let inactive = self.layout.inactive_slot()?;
        let binary_dest = self.layout.slot_path(inactive).join(&self.config.binary_name);

        if let Some(parent) = binary_dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(&download_path, &binary_dest).await?;
        tokio::fs::set_permissions(&binary_dest, std::fs::Permissions::from_mode(0o755)).await?;

        // Make the new binary executable and prepare for switch
        self.set_phase(Phase::AwaitingHealthCheck);
        self.state.active_slot = Some(inactive.as_str().to_string());
        self.save_state()?;

        // The new binary should be started externally. We wait for confirmation.
        // For now, we simulate: swap symlink and report success.
        self.set_phase(Phase::Switching);
        self.layout.swap_symlink(inactive)?;

        // Report success
        self.set_phase(Phase::ReportingSuccess);
        self.report_result(UpgradeStatus::Success, None).await?;

        // Cleanup download
        let _ = tokio::fs::remove_file(&download_path).await;

        let outcome = UpgradeOutcome::UpgradeComplete {
            target_version: metadata.latest_version.clone(),
        };
        self.set_phase(Phase::Idle);
        self.state.target_version = None;
        self.state.downloaded_file = None;
        self.save_state()?;

        tracing::info!(
            version = %metadata.latest_version,
            "upgrade complete"
        );

        Ok(outcome)
    }

    pub async fn run_loop(
        &mut self,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("shutdown signal received");
                        break;
                    }
                }
                _ = sleep(self.config.auto_upgrade_check_frequency) => {
                    match self.check_and_upgrade().await {
                        Ok(UpgradeOutcome::UpgradeComplete { target_version }) => {
                            tracing::info!(%target_version, "upgraded successfully");
                        }
                        Err(Error::AlreadyUpToDate { .. }) => {
                            tracing::debug!("already up to date");
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "upgrade check failed");
                            self.set_phase(Phase::Idle);
                            self.save_state()?;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn confirm_healthy(&mut self) -> Result<()> {
        if self.state.phase == Phase::AwaitingHealthCheck {
            self.set_phase(Phase::Switching);
            let slot_str = self.state.active_slot.as_deref().unwrap_or("slot_a");
            let slot = if slot_str == "slot_a" {
                Slot::A
            } else {
                Slot::B
            };
            self.layout.swap_symlink(slot)?;

            self.set_phase(Phase::ReportingSuccess);
            let _target_version = self.state.target_version.clone().unwrap_or_default();
            self.report_result(UpgradeStatus::Success, None).await?;

            if let Some(ref path) = self.state.downloaded_file {
                let _ = tokio::fs::remove_file(PathBuf::from(path)).await;
            }

            self.set_phase(Phase::Idle);
            self.state.target_version = None;
            self.state.downloaded_file = None;
            self.save_state()?;
        }
        Ok(())
    }

    fn set_phase(&mut self, phase: Phase) {
        self.state.transition(phase);
    }

    fn save_state(&self) -> Result<()> {
        self.state.save(&self.layout.state_file())
    }

    async fn report_result(&mut self, status: UpgradeStatus, error: Option<String>) -> Result<()> {
        if let Some(ref url) = self.config.state_report_url {
            let report = UpgradeReport {
                status,
                from_version: self.config.current_version.to_string(),
                to_version: self
                    .state
                    .target_version
                    .clone()
                    .unwrap_or_default(),
                error,
            };
            report_upgrade(&self.client, url, &report).await?;
        }
        Ok(())
    }

    fn target_platform(&self) -> &'static str {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "aarch64-apple-darwin";
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "x86_64-apple-darwin";
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return "aarch64-unknown-linux-gnu";
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return "x86_64-unknown-linux-gnu";
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return "x86_64-pc-windows-msvc";
        #[cfg(not(any(
            all(target_os = "macos", any(target_arch = "aarch64", target_arch = "x86_64")),
            all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64")),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        "unknown"
    }
}
