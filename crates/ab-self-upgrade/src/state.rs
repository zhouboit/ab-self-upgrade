use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::Slot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    CheckingForUpdate,
    Downloading,
    Verifying,
    Installing,
    AwaitingHealthCheck,
    Switching,
    ReportingSuccess,
    ReportingFailure,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpgradeState {
    pub phase: Phase,
    pub current_version: String,
    pub target_version: Option<String>,
    pub active_slot: Option<String>,
    pub downloaded_file: Option<String>,
    pub last_error: Option<String>,
}

impl UpgradeState {
    pub fn new(current_version: &str) -> Self {
        Self {
            phase: Phase::Idle,
            current_version: current_version.to_string(),
            target_version: None,
            active_slot: None,
            downloaded_file: None,
            last_error: None,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| Error::StateCorrupt {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn transition(&mut self, phase: Phase) {
        self.phase = phase;
    }
}

impl Slot {
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::A => "slot_a",
            Slot::B => "slot_b",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeOutcome {
    AlreadyUpToDate,
    UpgradeStarted { target_version: semver::Version },
    UpgradeComplete { target_version: semver::Version },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_state_roundtrip() {
        let state = UpgradeState {
            phase: Phase::Downloading,
            current_version: "0.1.0".to_string(),
            target_version: Some("0.2.0".to_string()),
            active_slot: Some("slot_b".to_string()),
            downloaded_file: None,
            last_error: None,
        };
        let tmp = NamedTempFile::new().unwrap();
        state.save(tmp.path()).unwrap();
        let loaded = UpgradeState::load(tmp.path()).unwrap();
        assert_eq!(loaded.phase, Phase::Downloading);
        assert_eq!(loaded.target_version.as_deref(), Some("0.2.0"));
        assert_eq!(loaded.active_slot.as_deref(), Some("slot_b"));
    }

    #[test]
    fn test_state_corrupt() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "not json").unwrap();
        let result = UpgradeState::load(tmp.path());
        assert!(matches!(result, Err(Error::StateCorrupt { .. })));
    }

    #[test]
    fn test_transition() {
        let mut state = UpgradeState::new("1.0.0");
        assert_eq!(state.phase, Phase::Idle);
        state.transition(Phase::CheckingForUpdate);
        assert_eq!(state.phase, Phase::CheckingForUpdate);
    }

    #[test]
    fn test_slot_as_str() {
        assert_eq!(Slot::A.as_str(), "slot_a");
        assert_eq!(Slot::B.as_str(), "slot_b");
    }
}
