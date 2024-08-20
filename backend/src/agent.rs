use futures_util::{SinkExt, StreamExt, TryFutureExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
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
            ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    error!("websocket send error: {}", e);
                })
                .await;
        }
    });

    agents
        .write()
        .await
        .insert(agent_key.clone(), AgentData { tx });

    while let Some(msg) = ws_rx.next().await {
        drop(msg);
    }

    disconnected(agent_key, &agents).await;
}

async fn disconnected(agent_key: String, agents: &Agents) {
    info!("good bye agent: {}", agent_key);

    agents.write().await.remove(&agent_key);
}
