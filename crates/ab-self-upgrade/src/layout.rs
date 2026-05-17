use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

const SLOT_A: &str = "slot_a";
const SLOT_B: &str = "slot_b";
const CURRENT_LINK: &str = "current";
const DOWNLOAD_DIR: &str = "download";
const STATE_FILE: &str = "state.json";

pub struct InstallLayout {
    root: PathBuf,
}

impl InstallLayout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn slot_a(&self) -> PathBuf {
        self.root.join(SLOT_A)
    }

    pub fn slot_b(&self) -> PathBuf {
        self.root.join(SLOT_B)
    }

    pub fn current_symlink(&self) -> PathBuf {
        self.root.join(CURRENT_LINK)
    }

    pub fn download_dir(&self) -> PathBuf {
        self.root.join(DOWNLOAD_DIR)
    }

    pub fn state_file(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    pub fn init_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.slot_a())?;
        std::fs::create_dir_all(self.slot_b())?;
        std::fs::create_dir_all(self.download_dir())?;

        if !self.current_symlink().exists() {
            std::os::unix::fs::symlink(self.slot_a(), self.current_symlink())?;
        }

        Ok(())
    }

    pub fn active_slot(&self) -> Result<Slot> {
        let target = std::fs::read_link(self.current_symlink())?;
        if target.ends_with(SLOT_A) {
            Ok(Slot::A)
        } else if target.ends_with(SLOT_B) {
            Ok(Slot::B)
        } else {
            Err(Error::Layout(format!(
                "current symlink points to unexpected target: {:?}",
                target
            )))
        }
    }

    pub fn inactive_slot(&self) -> Result<Slot> {
        match self.active_slot()? {
            Slot::A => Ok(Slot::B),
            Slot::B => Ok(Slot::A),
        }
    }

    pub fn slot_path(&self, slot: Slot) -> PathBuf {
        match slot {
            Slot::A => self.slot_a(),
            Slot::B => self.slot_b(),
        }
    }

    pub fn swap_symlink(&self, target_slot: Slot) -> Result<()> {
        let tmp_link = self.root.join(".current_tmp");
        let target_path = self.slot_path(target_slot);

        if tmp_link.exists() {
            std::fs::remove_file(&tmp_link)?;
        }
        std::os::unix::fs::symlink(&target_path, &tmp_link)?;
        std::fs::rename(&tmp_link, self.current_symlink())?;
        Ok(())
    }

    pub fn binary_path(&self, binary_name: &str) -> PathBuf {
        let slot = self.active_slot().unwrap_or(Slot::A);
        self.slot_path(slot).join(binary_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    A,
    B,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_and_active_slot() {
        let tmp = TempDir::new().unwrap();
        let layout = InstallLayout::new(tmp.path().to_path_buf());
        layout.init_dirs().unwrap();

        assert!(layout.slot_a().exists());
        assert!(layout.slot_b().exists());
        assert!(layout.download_dir().exists());
        assert_eq!(layout.active_slot().unwrap(), Slot::A);
        assert_eq!(layout.inactive_slot().unwrap(), Slot::B);
    }

    #[test]
    fn test_swap_symlink() {
        let tmp = TempDir::new().unwrap();
        let layout = InstallLayout::new(tmp.path().to_path_buf());
        layout.init_dirs().unwrap();

        assert_eq!(layout.active_slot().unwrap(), Slot::A);
        layout.swap_symlink(Slot::B).unwrap();
        assert_eq!(layout.active_slot().unwrap(), Slot::B);
        assert_eq!(layout.inactive_slot().unwrap(), Slot::A);

        layout.swap_symlink(Slot::A).unwrap();
        assert_eq!(layout.active_slot().unwrap(), Slot::A);
    }
}
