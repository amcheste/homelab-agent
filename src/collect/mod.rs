mod smart;
mod storage;
mod system;

use tokio::sync::mpsc;
use tokio::time::{interval, Duration, MissedTickBehavior};
use tracing::warn;

use crate::config::Config;
use crate::pb::{agent_message::Payload, AgentMessage};

/// Collection loop: gathers telemetry on the configured intervals and
/// hands it to the connection manager. Sending never blocks collection;
/// if the channel is full (control plane down long enough to fill the
/// buffer) the newest report is dropped and collection continues.
pub async fn run(cfg: Config, tx: mpsc::Sender<AgentMessage>) -> anyhow::Result<()> {
    let mut heartbeat = interval(Duration::from_secs(cfg.intervals.heartbeat_secs));
    let mut storage = interval(Duration::from_secs(cfg.intervals.storage_secs));
    let mut smart = interval(Duration::from_secs(cfg.intervals.smart_secs));
    for i in [&mut heartbeat, &mut storage, &mut smart] {
        i.set_missed_tick_behavior(MissedTickBehavior::Skip);
    }

    let mut collector = system::SystemCollector::new();

    // Inventory once at startup; the connection manager re-sends the
    // latest known inventory after each reconnect.
    send(&tx, Payload::Inventory(collector.inventory()));

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                send(&tx, Payload::Heartbeat(collector.heartbeat()));
            }
            _ = storage.tick() => {
                send(&tx, Payload::StorageUsage(storage::collect()));
            }
            _ = smart.tick() => {
                match smart::collect().await {
                    Ok(reports) => {
                        for report in reports {
                            send(&tx, Payload::SmartReport(report));
                        }
                    }
                    Err(err) => warn!(%err, "SMART collection failed"),
                }
            }
        }
    }
}

fn send(tx: &mpsc::Sender<AgentMessage>, payload: Payload) {
    let msg = AgentMessage {
        payload: Some(payload),
    };
    if let Err(err) = tx.try_send(msg) {
        warn!(%err, "report buffer full, dropping message");
    }
}
