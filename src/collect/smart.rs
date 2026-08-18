use anyhow::Context;
use serde_json::Value;
use tokio::process::Command;
use tracing::warn;

use crate::pb::SmartReport;

/// Collects SMART data by shelling out to smartctl (from smartmontools).
/// The raw JSON is relayed verbatim so the control plane can evolve its
/// parsing without an agent release; only the fields needed for basic
/// alerting are extracted here.
pub async fn collect() -> anyhow::Result<Vec<SmartReport>> {
    let mut reports = Vec::new();
    for device in scan_devices().await? {
        match report_for(&device).await {
            Ok(report) => reports.push(report),
            Err(err) => warn!(device, %err, "skipping device"),
        }
    }
    Ok(reports)
}

async fn scan_devices() -> anyhow::Result<Vec<String>> {
    let out = Command::new("smartctl")
        .args(["--scan-open", "--json"])
        .output()
        .await
        .context("running smartctl --scan-open (is smartmontools installed?)")?;
    let parsed: Value = serde_json::from_slice(&out.stdout).context("parsing scan output")?;
    Ok(parsed["devices"]
        .as_array()
        .map(|devs| {
            devs.iter()
                .filter_map(|d| d["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default())
}

async fn report_for(device: &str) -> anyhow::Result<SmartReport> {
    // smartctl exits non-zero for failing drives while still producing
    // valid JSON, so the exit status is deliberately not checked here.
    let out = Command::new("smartctl")
        .args(["--all", "--json", device])
        .output()
        .await
        .context("running smartctl")?;
    let raw = String::from_utf8_lossy(&out.stdout).into_owned();
    let parsed: Value = serde_json::from_str(&raw).context("parsing smartctl output")?;

    Ok(SmartReport {
        device: device.to_string(),
        healthy: parsed["smart_status"]["passed"].as_bool().unwrap_or(false),
        temperature_celsius: parsed["temperature"]["current"].as_i64().unwrap_or(0) as i32,
        power_on_hours: parsed["power_on_time"]["hours"].as_u64().unwrap_or(0),
        raw_json: raw,
    })
}
