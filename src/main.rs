mod collect;
mod config;
mod transport;

pub mod pb {
    tonic::include_proto!("homelab.agent.v1");
}

use anyhow::Context;
use clap::Parser;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Parser)]
#[command(version, about = "Homelab observability agent")]
struct Cli {
    /// Path to the agent config file
    #[arg(long, default_value = "/etc/homelab-agent/config.toml")]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "homelab_agent=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::load(&cli.config)
        .with_context(|| format!("loading config from {}", cli.config.display()))?;
    info!(endpoint = %cfg.control_plane.endpoint, "starting homelab-agent");

    // Collector -> connection manager. Bounded so a long control plane
    // outage applies backpressure instead of growing memory; the collector
    // drops the oldest pending report when full.
    let (tx, rx) = mpsc::channel::<pb::AgentMessage>(256);

    let collector = tokio::spawn(collect::run(cfg.clone(), tx));
    let connection = tokio::spawn(transport::run(cfg, rx));

    tokio::select! {
        _ = tokio::signal::ctrl_c() => info!("shutdown signal received"),
        res = collector => warn!(?res, "collector exited"),
        res = connection => warn!(?res, "connection manager exited"),
    }
    Ok(())
}
