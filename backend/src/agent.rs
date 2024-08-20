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

pub async fn connected(agent_key: String, ws: WebSocket, agents: Agents) {
    info!("Agent connected: {}", &agent_key);

    let (mut ws_tx, mut ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx: UnboundedReceiverStream<Message> = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            if ws_tx.send(message).await.is_err() {
                debug!("Exited receiver");
                return;
            };
        }
    });

    let our_tx = tx.clone();
    tokio::task::spawn(async move {
        let mut interval = interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            if our_tx.send(Message::ping([0])).is_err() {
                debug!("Exited Pinger");
                return;
            };
        }
    });

    agents
        .write()
        .await
        .insert(agent_key.clone(), AgentData { tx });

    while let Some(msg) = ws_rx.next().await {
        drop(msg);
    }

    disconnected(&agent_key, &agents).await;
}

async fn disconnected(agent_key: &str, agents: &Agents) {
    info!("good bye agent: {}", agent_key);

    agents.write().await.remove(agent_key);
}
