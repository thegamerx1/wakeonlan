extern crate dotenv;
extern crate pretty_env_logger;
extern crate serde;
#[macro_use]
extern crate log;

use dotenv::dotenv;
use ping_rs::{send_ping_async, PingOptions};
use tokio::time::interval;
use tokio::{fs, join, task};
use wol::{send_wol, MacAddr};

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
pub struct UserData {
    tx: mpsc::UnboundedSender<Message>,
    authenticated: bool,
}

type Users = Arc<RwLock<HashMap<usize, UserData>>>;
type Devices = Arc<RwLock<Vec<Device>>>;

static DEVICES_PATH: &'static str = "data/devices.json";

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    _ = fs::create_dir("data/").await;

    let devices_path = Path::new(DEVICES_PATH);

    if !devices_path.exists() {
        fs::write(devices_path, "[]")
            .await
            .expect("Write default devices.json");
    }

    let devices: Devices = Arc::new(RwLock::new(
        serde_json::from_str::<Vec<Device>>(
            fs::read_to_string(devices_path).await.unwrap().as_str(),
        )
        .unwrap(),
    ));

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Users::default();
    // Turn our "state" into a new Filter...
    let back_devices = Arc::clone(&devices);
    let back_users = Arc::clone(&users);
    let users_warp = warp::any().map(move || users.clone());
    let devices_warp = warp::any().map(move || devices.clone());

    // GET /ws -> websocket upgrade
    let gateway = warp::path("ws")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(users_warp)
        .and(devices_warp)
        .map(|ws: warp::ws::Ws, users, devices| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| user_connected(socket, users, devices))
        });

    let health_check = warp::path("health-check").map(|| format!("Server OK"));
    let html_file = warp::path("login").and(warp::fs::file("public/index.html"));
    let public = warp::get().and(warp::fs::dir("public/"));
    let routes = health_check.or(gateway.or(html_file.or(public)));

    let background_task = task::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            let devices_count = &back_devices.read().await.len();
            let user_count = &back_users.read().await.len();
            if user_count > &0 && devices_count > &0 {
                let result = ping_hosts(&back_devices).await;
                broadcast(WsResponse::Update { data: result }, &back_users).await;
            }
        }
    });

    let port = std::env::var("APP_PORT")
        .unwrap_or("9000".to_owned())
        .parse::<u16>()
        .unwrap();
    let host = std::env::var("APP_HOST")
        .unwrap_or("127.0.0.1".to_owned())
        .parse::<IpAddr>()
        .unwrap();

    _ = join!(
        warp::serve(routes).run(SocketAddr::new(host, port)),
        background_task
    );
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Device {
    name: String,
    mac: String,
    host: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event")]
enum WsRequest {
    #[serde(rename = "login")]
    Login { login: String, nonce: i32 },
    #[serde(rename = "wake")]
    Wake { mac: String, nonce: i32 },
    #[serde(rename = "save")]
    Save { devices: Vec<Device>, nonce: i32 },
}

pub type LatencyDevices = HashMap<String, Option<u32>>;

#[derive(Serialize, Deserialize)]
#[serde(tag = "event")]
enum WsResponse {
    #[serde(rename = "wake")]
    Wake { success: bool, nonce: i32 },
    #[serde(rename = "save")]
    Save { success: bool, nonce: i32 },
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

async fn user_connected(ws: WebSocket, users: Users, devices: Devices) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    info!("new user: {}", my_id);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
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

    while let Some(result) = user_ws_rx.next().await {
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
            Err(_) => {
                debug!("{:?}", msg);
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
                _ => WsResponse::Error {
                    nonce: get_nonce(req),
                },
            };
            send_event(&my_user, response).await;
        };
    }

    // user_message(my_id, msg, &users, &devices).await;

    user_disconnected(my_id, &users).await;
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
        Err(err) => {
            error!("{:?}", err);
            None
        }
    }
}

fn get_nonce(msg: WsRequest) -> i32 {
    match msg {
        WsRequest::Save { devices: _, nonce } => nonce,
        WsRequest::Wake { mac: _, nonce } => nonce,
        WsRequest::Login { login: _, nonce } => nonce,
    }
}

async fn ping_hosts(devices: &Arc<tokio::sync::RwLock<Vec<Device>>>) -> LatencyDevices {
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

async fn broadcast(msg: WsResponse, users: &Users) {
    let message = Message::text(serde_json::to_string(&msg).unwrap());

    for (&_uid, data) in users.read().await.iter() {
        if data.authenticated {
            _ = data.tx.send(message.clone());
        }
    }
}

async fn user_disconnected(my_id: usize, users: &Users) {
    info!("good bye user: {}", my_id);

    users.write().await.remove(&my_id);
}
