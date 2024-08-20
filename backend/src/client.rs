use futures_util::{SinkExt, StreamExt, TryFutureExt};
use ping_rs::{send_ping_async, PingOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use wol::{send_wol, MacAddr};

use crate::agent::Agents;
use crate::DEVICES_PATH;

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
pub struct UserData {
    tx: mpsc::UnboundedSender<Message>,
    authenticated: bool,
}

pub type Users = Arc<RwLock<HashMap<usize, UserData>>>;
pub type Devices = Arc<RwLock<Vec<Device>>>;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Serialize, Deserialize, Clone)]
pub struct Device {
    pub name: String,
    pub mac: String,
    pub host: String,
    pub api_key: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum WsRequest {
    #[serde(rename = "login")]
    Login { login: String, nonce: i32 },
    #[serde(rename = "wake")]
    Wake { mac: String, nonce: i32 },
    #[serde(rename = "shutdown")]
    Shutdown { key: String, nonce: i32 },
    #[serde(rename = "save")]
    Save { devices: Vec<Device>, nonce: i32 },
}

pub type LatencyDevices = HashMap<String, Option<u32>>;

#[derive(Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum WsResponse {
    #[serde(rename = "wake")]
    Wake { success: bool, nonce: i32 },
    #[serde(rename = "save")]
    Save { success: bool, nonce: i32 },
    #[serde(rename = "shutdown")]
    Shutdown { success: bool, nonce: i32 },
    #[serde(rename = "login")]
    Login {
        success: bool,
        devices: Option<Vec<Device>>,
        nonce: i32,
    },
    #[serde(rename = "update")]
    Update { data: LatencyDevices },
    #[serde(rename = "devices")]
    Devices { data: Vec<Device> },
    #[serde(rename = "error")]
    Error { nonce: i32 },
}

pub async fn connected(ws: WebSocket, users: Users, devices: Devices, agents: Agents) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    info!("new user: {}", my_id);

    let (mut ws_tx, mut ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

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

    // Save the sender in our list of connected users.
    users.write().await.insert(
        my_id,
        UserData {
            authenticated: false,
            tx,
        },
    );

    // tx.send(LoginEvent {
    //     devices: vec![],
    //     event: "login",
    //     success: true,
    // });

    while let Some(result) = ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        let req: WsRequest = match serde_json::from_str(match msg.to_str() {
            Err(_) => {
                debug!("{:?}", msg);
                error!("Error converting message to str");
                continue;
            }
            Ok(msg) => msg,
        }) {
            Err(err) => {
                debug!("{:?}", msg);
                error!("{:?}", err);
                error!("Received invalid json");
                continue;
            }
            Ok(val) => val,
        };

        let _users = users.read().await;
        let my_user = _users.get(&my_id).unwrap();

        let response: WsResponse;

        if !my_user.authenticated {
            drop(_users);
            let mut _users = users.write().await;
            let my_user = _users.get_mut(&my_id).unwrap();
            response = match req {
                WsRequest::Login { login, nonce } => {
                    if login == std::env::var("APP_KEY").unwrap() {
                        my_user.authenticated = true;
                    }
                    WsResponse::Login {
                        success: my_user.authenticated,
                        devices: if my_user.authenticated {
                            Some(devices.read().await.to_vec())
                        } else {
                            None
                        },
                        nonce,
                    }
                }
                _ => WsResponse::Error {
                    nonce: get_nonce(req),
                },
            };
            send_event(&my_user, response).await;
        } else {
            response = match req {
                WsRequest::Save {
                    devices: device_list,
                    nonce,
                } => {
                    let mut devices = devices.write().await;
                    WsResponse::Save {
                        nonce,
                        success: match tokio::fs::write(
                            DEVICES_PATH,
                            serde_json::to_string_pretty(&device_list).unwrap(),
                        )
                        .await
                        {
                            Ok(_) => {
                                broadcast(
                                    WsResponse::Devices {
                                        data: device_list.clone(),
                                    },
                                    &users,
                                )
                                .await;
                                *devices = device_list;
                                true
                            }
                            Err(_) => false,
                        },
                    }
                }
                WsRequest::Wake { mac, nonce } => match MacAddr::from_str(mac.as_str()) {
                    Ok(addr) => WsResponse::Wake {
                        nonce,
                        success: match send_wol(addr, None, None) {
                            Err(_) => false,
                            Ok(_) => true,
                        },
                    },
                    Err(_) => WsResponse::Wake {
                        nonce,
                        success: false,
                    },
                },
                WsRequest::Shutdown { key, nonce } => {
                    let agents = agents.read().await;
                    WsResponse::Shutdown {
                        success: match agents.get(&key) {
                            Some(agent) => {
                                warn!("Sending shutdown message to agent {}", &key);
                                match agent.tx.send(Message::text("suspend")) {
                                    Ok(()) => true,
                                    Err(err) => {
                                        error!("Could not send message to agent {err}");
                                        false
                                    }
                                }
                            }
                            None => false,
                        },
                        nonce: nonce,
                    }
                }
                _ => WsResponse::Error {
                    nonce: get_nonce(req),
                },
            };
            send_event(&my_user, response).await;
        };
    }

    // user_message(my_id, msg, &users, &devices).await;

    disconnected(my_id, &users).await;
}

const PING_OPTS: PingOptions = PingOptions {
    ttl: 128,
    dont_fragment: true,
};
const DURATION: Duration = Duration::from_secs(1);
const BUFFER: &[u8; 8] = &[8; 8];
async fn ping_host(host: IpAddr) -> Option<u32> {
    match send_ping_async(&host, DURATION, Arc::new(BUFFER), Some(&PING_OPTS)).await {
        Ok(result) => Some(result.rtt),
        Err(_err) => None,
    }
}

fn get_nonce(msg: WsRequest) -> i32 {
    match msg {
        WsRequest::Save { devices: _, nonce } => nonce,
        WsRequest::Wake { mac: _, nonce } => nonce,
        WsRequest::Login { login: _, nonce } => nonce,
        WsRequest::Shutdown { key: _, nonce } => nonce,
    }
}

pub async fn ping_hosts(devices: &Arc<tokio::sync::RwLock<Vec<Device>>>) -> LatencyDevices {
    let mut result = HashMap::new();
    for device in devices.read().await.iter() {
        let ip = match IpAddr::from_str(device.host.as_str()) {
            Ok(ip) => ip,
            Err(_) => {
                result.insert(device.host.to_owned(), None);
                continue;
            }
        };

        result.insert(device.host.to_owned(), ping_host(ip).await);
    }
    result
}

async fn send_event(user: &UserData, msg: WsResponse) {
    match user
        .tx
        .send(Message::text(serde_json::to_string(&msg).unwrap()))
    {
        Ok(_) => (),
        Err(err) => error!("{err}"),
    };
}

pub async fn broadcast(msg: WsResponse, users: &Users) {
    let message = Message::text(serde_json::to_string(&msg).unwrap());

    for (&_uid, data) in users.read().await.iter() {
        if data.authenticated {
            _ = data.tx.send(message.clone()).ok();
        }
    }
}

async fn disconnected(my_id: usize, users: &Users) {
    info!("good bye user: {}", my_id);

    users.write().await.remove(&my_id);
}
