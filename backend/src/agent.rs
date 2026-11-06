use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::filters::ws::{Message, WebSocket};

pub struct AgentData {
    pub tx: mpsc::UnboundedSender<Message>,
}

pub type Agents = Arc<RwLock<HashMap<String, AgentData>>>;

const HEARTBEAT: Duration = Duration::from_secs(15);
const TIMEOUT: Duration = Duration::from_secs(20);

pub async fn connected(agent_key: String, ws: WebSocket, agents: Agents) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx: UnboundedReceiverStream<Message> = UnboundedReceiverStream::new(rx);

    let writer = tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            if ws_tx.send(message).await.is_err() {
                debug!("Exited receiver");
                return;
            };
        }
    });

    let our_tx = tx.clone();
    let pinger = tokio::task::spawn(async move {
        let mut interval = interval(HEARTBEAT);
        loop {
            interval.tick().await;
            if our_tx.send(Message::ping([0])).is_err() {
                debug!("Exited Pinger");
                return;
            };
        }
    });

    let previous = agents
        .write()
        .await
        .insert(agent_key.clone(), AgentData { tx: tx.clone() });
    if previous.is_some() {
        info!(
            "Agent reconnected: {} (replacing previous connection)",
            agent_key
        );
    } else {
        info!("Agent connected: {}", agent_key);
    }

    let reason = loop {
        match tokio::time::timeout(TIMEOUT, ws_rx.next()).await {
            Ok(Some(Ok(message))) if message.is_close() => break "peer closed".to_owned(),
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(err))) => break format!("websocket error: {err}"),
            Ok(None) => break "connection ended".to_owned(),
            Err(_) => break format!("no heartbeat for {} seconds", TIMEOUT.as_secs()),
        }
    };

    writer.abort();
    pinger.abort();
    disconnected(&agent_key, &agents, &tx, &reason).await;
}

async fn disconnected(
    agent_key: &str,
    agents: &Agents,
    tx: &mpsc::UnboundedSender<Message>,
    reason: &str,
) {
    let mut agents = agents.write().await;
    // A replaced connection must not remove the newer agent for the same host.
    if agents
        .get(agent_key)
        .is_some_and(|agent| agent.tx.same_channel(tx))
    {
        agents.remove(agent_key);
        info!("good bye agent: {} ({})", agent_key, reason);
    } else {
        debug!(
            "Previous agent connection closed: {} ({})",
            agent_key, reason
        );
    }
}
