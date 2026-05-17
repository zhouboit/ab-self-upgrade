use reqwest::Client;
use serde::Serialize;

use crate::error::Result;

#[derive(Debug, Serialize)]
pub struct UpgradeReport {
    pub status: UpgradeStatus,
    pub from_version: String,
    pub to_version: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeStatus {
    Success,
    Failed,
}

pub async fn report_upgrade(
    client: &Client,
    url: &str,
    report: &UpgradeReport,
) -> Result<()> {
    client
        .post(url)
        .json(report)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
