use std::path::Path;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub control_plane: ControlPlane,
    #[serde(default)]
    pub intervals: Intervals,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPlane {
    /// gRPC endpoint, e.g. "https://control.lab.example:9443"
    pub endpoint: String,
    /// Path to the one-time enrollment token; consumed on first contact.
    #[serde(default = "default_token_path")]
    #[allow(dead_code, reason = "read once enrollment is wired in transport")]
    pub enrollment_token_path: String,
    /// Where the per-node credential from Enroll is persisted.
    #[serde(default = "default_credential_path")]
    #[allow(dead_code, reason = "read once enrollment is wired in transport")]
    pub credential_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intervals {
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
    #[serde(default = "default_storage_secs")]
    pub storage_secs: u64,
    #[serde(default = "default_smart_secs")]
    pub smart_secs: u64,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            heartbeat_secs: default_heartbeat_secs(),
            storage_secs: default_storage_secs(),
            smart_secs: default_smart_secs(),
        }
    }
}

fn default_token_path() -> String {
    "/etc/homelab-agent/enrollment-token".into()
}

fn default_credential_path() -> String {
    "/var/lib/homelab-agent/credential".into()
}

fn default_heartbeat_secs() -> u64 {
    30
}

fn default_storage_secs() -> u64 {
    300
}

fn default_smart_secs() -> u64 {
    3600
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config_gets_defaults() {
        let cfg: Config = toml::from_str(
            r#"
            [control_plane]
            endpoint = "https://control.lab:9443"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.intervals.heartbeat_secs, 30);
        assert_eq!(cfg.intervals.smart_secs, 3600);
        assert_eq!(
            cfg.control_plane.credential_path,
            "/var/lib/homelab-agent/credential"
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let err = toml::from_str::<Config>(
            r#"
            [control_plane]
            endpoint = "https://control.lab:9443"
            typo_field = true
            "#,
        );
        assert!(err.is_err());
    }
}
