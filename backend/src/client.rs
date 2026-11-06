use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, AtomicUsize};
use std::sync::{atomic::Ordering, Arc};
use std::time::Duration;
use surge_ping::{Client as PingClient, Config as PingConfig, PingIdentifier, PingSequence, ICMP};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::http::StatusCode;
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
    Shutdown { host: String, nonce: i32 },
    #[serde(rename = "save")]
    Save { devices: Vec<Device>, nonce: i32 },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceData {
    pub latency: Option<u32>,
    pub connected: bool,
}

pub type UpdateDevices = HashMap<String, DeviceData>;

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
    Update { data: UpdateDevices },
    #[serde(rename = "devices")]
    Devices { data: Vec<Device> },
    #[serde(rename = "error")]
    Error { nonce: i32 },
}

pub async fn send_shutdown(agents: &Agents, host: &String) -> bool {
    let agents = agents.read().await;
    match agents.get(host) {
        Some(agent) => {
            warn!("Sending shutdown message to agent {}", &host);
            match agent.tx.send(Message::text("suspend")) {
                Ok(()) => true,
                Err(err) => {
                    error!("Could not send message to agent {err}");
                    false
                }
            }
        }
        None => false,
    }
}

#[derive(Deserialize)]
enum APIAction {
    #[serde(rename = "wake")]
    Wake,
    #[serde(rename = "sleep")]
    Sleep,
}

#[derive(Deserialize)]
pub struct InputData {
    action: APIAction,
    name: String,
}

pub async fn api_call(
    data: InputData,
    auth_header: Option<String>,
    devices: Devices,
    agents: Agents,
) -> Result<impl warp::Reply, warp::Rejection> {
    match auth_header {
        Some(auth) if auth == std::env::var("APP_KEY").unwrap() => {}
        _ => {
            return Ok(warp::reply::with_status(
                "Unauthorized",
                StatusCode::UNAUTHORIZED,
            ))
        }
    };

    let read = devices.read().await;

    let device = match read.iter().find(|dev| dev.name == data.name) {
        None => {
            return Ok(warp::reply::with_status(
                "Device not found",
                StatusCode::NOT_FOUND,
            ))
        }
        Some(dev) => dev,
    };

    let success = match data.action {
        APIAction::Sleep => send_shutdown(&agents, &device.host).await,
        APIAction::Wake => {
            MacAddr::from_str(&device.mac).is_ok_and(|address| send_wol(address, None, None).is_ok())
        }
    };

    return if success {
        Ok(warp::reply::with_status("OK", StatusCode::OK))
    } else {
        Ok(warp::reply::with_status(
            "Failed",
            StatusCode::FAILED_DEPENDENCY,
        ))
    };
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
            if let Err(e) = ws_tx.send(message).await {
                error!("websocket send error: {}", e);
                break;
            }
        }
        // Removing the user's sender ends this loop; flush the close handshake.
        let _ = ws_tx.close().await;
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
        if msg.is_close() {
            break;
        }
        // Warp replies to Ping frames automatically. Only text frames contain requests.
        let Ok(text) = msg.to_str() else {
            continue;
        };
        let req: WsRequest = match serde_json::from_str(text) {
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
                WsRequest::Shutdown { host, nonce } => WsResponse::Shutdown {
                    success: send_shutdown(&agents, &host).await,
                    nonce: nonce,
                },
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

const DURATION: Duration = Duration::from_secs(1);
const BUFFER: &[u8; 8] = &[8; 8];
static NEXT_PING_ID: AtomicU16 = AtomicU16::new(0);

fn ping_client(kind: ICMP) -> Option<PingClient> {
    let config = PingConfig::builder().kind(kind).ttl(128).build();
    match PingClient::new(&config) {
        Ok(client) => Some(client),
        Err(err) => {
            debug!("Cannot create {:?} ping socket: {}", kind, err);
            None
        }
    }
}

async fn ping_host(client: &PingClient, host: IpAddr) -> Option<u32> {
    let id = NEXT_PING_ID.fetch_add(1, Ordering::Relaxed);
    let mut pinger = client.pinger(host, PingIdentifier(id)).await;
    pinger.timeout(DURATION);
    match pinger.ping(PingSequence(id), BUFFER).await {
        Ok((_, elapsed)) => Some(elapsed.as_millis().min(u32::MAX as u128) as u32),
        Err(err) => {
            debug!("ICMP ping failed for {}: {:?}", host, err);
            None
        }
    }
}

fn get_nonce(msg: WsRequest) -> i32 {
    match msg {
        WsRequest::Save { devices: _, nonce } => nonce,
        WsRequest::Wake { mac: _, nonce } => nonce,
        WsRequest::Login { login: _, nonce } => nonce,
        WsRequest::Shutdown { host: _, nonce } => nonce,
    }
}

pub async fn ping_hosts(devices: &[Device], agents: &Agents) -> UpdateDevices {
    // Bound concurrent probes and do not hold the registry lock during network I/O.
    let hosts: Vec<_> = devices
        .iter()
        .map(|device| (device.host.clone(), IpAddr::from_str(&device.host).ok()))
        .collect();
    // Reuse one socket per address family for all probes in this monitoring round.
    let ipv4 = hosts
        .iter()
        .any(|(_, ip)| matches!(ip, Some(IpAddr::V4(_))))
        .then(|| ping_client(ICMP::V4))
        .flatten();
    let ipv6 = hosts
        .iter()
        .any(|(_, ip)| matches!(ip, Some(IpAddr::V6(_))))
        .then(|| ping_client(ICMP::V6))
        .flatten();
    let result: Vec<_> = futures_util::stream::iter(hosts.into_iter().map(|(host, ip)| {
        let client = match ip {
            Some(IpAddr::V4(_)) => ipv4.clone(),
            Some(IpAddr::V6(_)) => ipv6.clone(),
            None => None,
        };
        async move {
            let latency = match (ip, client) {
                (Some(ip), Some(client)) => ping_host(&client, ip).await,
                _ => None,
            };
            (host, latency)
        }
    }))
    .buffer_unordered(16)
    .collect()
    .await;
    let agents = agents.read().await;
    result
        .into_iter()
        .map(|(host, latency)| {
            let connected = agents.contains_key(&host);
            (host, DeviceData { latency, connected })
        })
        .collect()
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shared_icmp_socket_handles_concurrent_loopback_probes() {
        let client = ping_client(ICMP::V4).expect("Create ICMP socket");
        let host = IpAddr::from_str("127.0.0.1").unwrap();
        let (first, second) = tokio::join!(ping_host(&client, host), ping_host(&client, host));
        assert!(first.is_some(), "First loopback echo must receive a reply");
        assert!(
            second.is_some(),
            "Second loopback echo must receive a reply"
        );
    }
}
