use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse metadata: {0}")]
    MetadataParse(#[from] serde_json::Error),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("failed to parse version: {0}")]
    VersionParse(#[from] semver::Error),

    #[error("upgrade state corrupt at {path}: {reason}")]
    StateCorrupt { path: PathBuf, reason: String },

    #[error("already up to date (current: {current}, latest: {latest})")]
    AlreadyUpToDate { current: semver::Version, latest: semver::Version },

    #[error("no platform match for target: {0}")]
    NoPlatformMatch(String),

    #[error("download failed for {url}: {reason}")]
    DownloadFailed { url: String, reason: String },

    #[error("layout error: {0}")]
    Layout(String),
}

pub type Result<T> = std::result::Result<T, Error>;
