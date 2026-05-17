use std::path::PathBuf;

use ab_self_upgrade::UpgradeConfig;
use ab_self_upgrade::UpgradeEngine;
use ab_self_upgrade::Version;

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

    let mut engine = UpgradeEngine::new(config).expect("failed to create engine");

    println!("Initial state: {:?}", engine.state().phase);

    match engine.check_and_upgrade().await {
        Ok(outcome) => println!("Upgrade outcome: {:?}", outcome),
        Err(ab_self_upgrade::Error::AlreadyUpToDate { current, latest }) => {
            println!("Already up to date: {} (latest: {})", current, latest);
        }
        Err(e) => {
            println!("Upgrade check result (expected in example): {}", e);
        }
    }

    println!("Final state: {:?}", engine.state().phase);

    // To run the periodic loop instead:
    // let (tx, rx) = watch::channel(false);
    // tokio::spawn(async move {
    //     let _ = engine.run_loop(rx).await;
    // });
    // // On shutdown: tx.send(true).unwrap();
}
