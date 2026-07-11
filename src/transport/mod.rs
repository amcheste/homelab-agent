use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tracing::{info, warn};

use crate::config::Config;
use crate::pb::agent_gateway_client::AgentGatewayClient;
use crate::pb::{control_message::Payload, AgentMessage};

const BACKOFF_INITIAL: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Connection manager: dials the control plane, holds the bidirectional
/// stream, and forwards collector output upstream. Reconnects with capped
/// exponential backoff on any failure.
///
/// TODO(enrollment): exchange the one-time token via Enroll on first
/// contact, persist the per-node credential, and attach it to Stream
/// as request metadata. Until the control plane exists this connects
/// unauthenticated.
pub async fn run(cfg: Config, mut rx: mpsc::Receiver<AgentMessage>) -> anyhow::Result<()> {
    let mut backoff = BACKOFF_INITIAL;
    loop {
        match connect_and_stream(&cfg, &mut rx).await {
            Ok(()) => {
                info!("stream closed cleanly, reconnecting");
                backoff = BACKOFF_INITIAL;
            }
            Err(err) => {
                warn!(%err, next_retry_secs = backoff.as_secs(), "connection failed");
                sleep(backoff).await;
                backoff = (backoff * 2).min(BACKOFF_MAX);
            }
        }
    }
}

async fn connect_and_stream(
    cfg: &Config,
    rx: &mut mpsc::Receiver<AgentMessage>,
) -> anyhow::Result<()> {
    let channel = Channel::from_shared(cfg.control_plane.endpoint.clone())?
        .connect_timeout(Duration::from_secs(10))
        .connect()
        .await?;
    let mut client = AgentGatewayClient::new(channel);
    info!(endpoint = %cfg.control_plane.endpoint, "connected to control plane");

    // Bridge the collector channel into the request stream. A second
    // small channel lets this function keep ownership of `rx` across
    // reconnects.
    let (stream_tx, stream_rx) = mpsc::channel::<AgentMessage>(16);
    let response = client.stream(ReceiverStream::new(stream_rx)).await?;
    let mut inbound = response.into_inner();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                let Some(msg) = msg else {
                    return Ok(()); // collector shut down
                };
                if stream_tx.send(msg).await.is_err() {
                    anyhow::bail!("outbound stream closed by control plane");
                }
            }
            inbound_msg = inbound.message() => {
                match inbound_msg? {
                    Some(ctrl) => handle_control(ctrl.payload),
                    None => return Ok(()), // server ended the stream
                }
            }
        }
    }
}

fn handle_control(payload: Option<Payload>) {
    // Phase 1: control messages are informational only. Phase 2 actuation
    // gets its own dispatch with per-action authorization.
    match payload {
        Some(Payload::Ping(p)) => {
            info!(sent_at = p.sent_at_unix_seconds, "ping from control plane")
        }
        Some(Payload::Ack(a)) => info!(detail = %a.detail, "ack from control plane"),
        None => warn!("control message with empty payload"),
    }
}
