use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

pub fn verify_sha256(path: &Path, expected_hex: &str) -> Result<()> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let actual = hex::encode(hasher.finalize());

    if actual == expected_hex {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_verify_sha256_ok() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        // echo -n "hello world" | sha256sum
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256(tmp.path(), expected).is_ok());
    }

    #[test]
    fn test_verify_sha256_mismatch() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let result = verify_sha256(tmp.path(), "0000deadbeef");
        assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
    }
}
