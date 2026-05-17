# A/B Self Upgrade

A Rust library that enables applications to automatically download, verify, and install new versions while keeping the old binary running until the new one confirms healthy.

## How It Works

1. The client periodically fetches version metadata from a cloud endpoint.
2. If a newer version is available, it downloads the binary for the current platform.
3. The downloaded file's SHA-256 checksum is verified against the metadata.
4. The new binary is installed into the inactive A/B slot.
5. A symlink (`current`) is atomically swapped to point to the new slot.
6. The old binary keeps running until the new one starts and confirms healthy.
7. Upgrade success or failure is reported to the cloud.

### A/B Directory Layout

```
<install_dir>/
├── current -> slot_a       # symlink to active slot
├── slot_a/
│   └── <binary_name>
├── slot_b/
│   └── <binary_name>
├── download/               # staging for new downloads
└── state.json              # persisted upgrade state
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ab-self-upgrade = { path = "crates/ab-self-upgrade" }
tokio = { version = "1", features = ["full"] }
```

```rust
use ab_self_upgrade::{UpgradeConfig, UpgradeEngine, Version};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let config = UpgradeConfig::from_str(r#"
        api_domain = https://cloud.example.com
        auto_upgrade_enabled = true
        auto_upgrade_metadata = /releases/meta.json
        auto_upgrade_check_frequency = 10
    "#)
    .unwrap()
    .with_install_dir(PathBuf::from("/opt/myapp/upgrade"))
    .with_binary_name("myapp")
    .with_current_version(Version::parse("1.0.0").unwrap())
    .with_state_report_url("https://cloud.example.com/upgrade/report");

    let mut engine = UpgradeEngine::new(config).unwrap();

    // Single upgrade check
    match engine.check_and_upgrade().await {
        Ok(outcome) => println!("Upgrade: {:?}", outcome),
        Err(ab_self_upgrade::Error::AlreadyUpToDate { .. }) => {}
        Err(e) => eprintln!("Upgrade failed: {}", e),
    }

    // Or run as a periodic background loop
    // let (tx, rx) = tokio::sync::watch::channel(false);
    // tokio::spawn(async move { engine.run_loop(rx).await });
    // tx.send(true).unwrap(); // shutdown
}
```

## Cloud Metadata Format

The metadata endpoint must return JSON with this schema:

```json
{
  "latest_version": "1.2.3",
  "platforms": {
    "x86_64-apple-darwin": {
      "download_url": "/releases/v1.2.3/app-x86_64",
      "checksum": "sha256:<hex>"
    },
    "aarch64-unknown-linux-gnu": {
      "download_url": "/releases/v1.2.3/app-aarch64",
      "checksum": "sha256:<hex>"
    }
  }
}
```

The download URL is resolved as `${api_domain}${download_url}`.

Supported target triples: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`.

## State Reporting

If `state_report_url` is configured, the library POSTs upgrade results:

```json
{
  "status": "success",
  "from_version": "1.0.0",
  "to_version": "1.2.3",
  "error": null
}
```

## Build & Test

```bash
cargo build                            # build workspace
cargo test                             # run all tests
cargo test -p ab-self-upgrade          # library tests only
cargo run -p ab-self-upgrade-example   # run example binary
```

## License

MIT
