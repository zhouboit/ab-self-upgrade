use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ab_self_upgrade::UpgradeConfig;
use ab_self_upgrade::UpgradeEngine;
use ab_self_upgrade::Version;
use tokio::sync::{watch, Mutex};

const CONFIG: &str = r#"
api_domain = https://cloud.example.com
auto_upgrade_enabled = true
auto_upgrade_metadata = /releases/meta.json
auto_upgrade_check_frequency = 1
"#;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let config = UpgradeConfig::from_str(CONFIG)
        .expect("failed to parse config")
        .with_install_dir(PathBuf::from("/tmp/ab-self-upgrade-example"))
        .with_binary_name("my-app")
        .with_current_version(Version::new(0, 1, 0))
        .with_state_report_url("https://cloud.example.com/upgrade/report".to_string());

    println!("Starting A/B self-upgrade example");
    println!("  Current version: {}", config.current_version);
    println!("  API domain: {}", config.api_domain);
    println!("  Install dir: {:?}", config.install_dir);
    println!("  Metadata URL: {}", config.metadata_url());
    println!();

    let engine = Arc::new(Mutex::new(
        UpgradeEngine::new(config).expect("failed to create engine"),
    ));

    println!("Initial state: {:?}", engine.lock().await.state().phase);

    // Start the upgrade loop in the background.
    // It runs periodically and stops when the program exits (or shutdown signal).
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let _loop_handle = UpgradeEngine::spawn_run_loop(engine.clone(), shutdown_rx);

    println!("Upgrade loop running in background...");
    println!("Main program is free to do other work.");
    println!();

    // Simulate the main program doing work.
    // The upgrade loop checks for updates in the background automatically.
    for i in 1..=3 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let phase = engine.lock().await.state().phase;
        println!("[main] working... (tick {}) — upgrade phase: {:?}", i, phase);
    }

    println!();
    println!("Main program shutting down, sending stop signal to upgrade loop...");

    // Graceful shutdown (optional — the loop also stops when the tokio runtime drops)
    shutdown_tx.send(true).unwrap();

    println!("Done.");
}
