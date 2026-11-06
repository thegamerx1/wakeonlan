extern crate dotenv;
extern crate serde;
#[macro_use]
extern crate log;

use agent::Agents;
use client::{Device, Devices, Users};
use dotenv::dotenv;
use serde::Serialize;
use tokio::time::interval;
use tokio::{fs, task};

use std::convert::Infallible;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{watch, RwLock};
use warp::http::StatusCode;
use warp::Filter;

use crate::client::api_call;

static DEVICES_PATH: &'static str = "data/devices.json";

#[derive(Error, Debug)]
enum ApiErrors {
    #[error("user not authorized")]
    NotAuthorized(String),
}

#[derive(Serialize, Debug)]
struct ApiErrorResult {
    detail: String,
}

// ensure that warp`s Reject recognizes `ApiErrors`
impl warp::reject::Reject for ApiErrors {}

mod agent;
mod client;
mod mqtt;

#[tokio::main]
async fn main() {
    dotenv().ok();
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    if std::env::var("APP_KEY").is_err() {
        eprintln!("APP_KEY not set");
        exit(1);
    }

    pretty_env_logger::init_timed();

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

    let users = Users::default();
    let agents = Agents::default();
    let mqtt_config = mqtt::Config::from_env().await.unwrap_or_else(|err| {
        error!("Invalid MQTT configuration: {err}");
        exit(1);
    });
    let mqtt_enabled = mqtt_config.is_some();
    let (stop_tx, stop_rx) = watch::channel(false);
    let (snapshots_tx, snapshots_rx) = watch::channel(None);
    let mqtt_task = mqtt_config.map(|config| {
        task::spawn(mqtt::run(
            config,
            snapshots_rx,
            devices.clone(),
            agents.clone(),
            stop_rx.clone(),
        ))
    });
    let back_devices = Arc::clone(&devices);
    let agents2 = Arc::clone(&agents);
    let back_devices2 = Arc::clone(&devices);
    let api_devices = Arc::clone(&devices);
    let api_agents = Arc::clone(&agents);
    let back_users = Arc::clone(&users);
    let back_agents = Arc::clone(&agents);
    let users_warp = warp::any().map(move || users.clone());
    let devices_warp = warp::any().map(move || back_devices2.clone());
    let agents_warp = warp::any().map(move || agents.clone());
    let agents_warp2 = warp::any().map(move || agents2.clone());
    let devices_agent = warp::any().map(move || devices.clone());
    let api_devices_warp = warp::any().map(move || api_devices.clone());
    let api_agents_warp = warp::any().map(move || api_agents.clone());

    let api = warp::path("api")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("Authorization"))
        .and(api_devices_warp)
        .and(api_agents_warp)
        .and_then(api_call);

    // GET /ws -> websocket upgrade
    let gateway = warp::path("ws")
        .and(warp::ws())
        .and(users_warp)
        .and(devices_warp)
        .and(agents_warp2)
        .map(|ws: warp::ws::Ws, users, devices, agents| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| client::connected(socket, users, devices, agents))
        });

    let agent_gateway = warp::path("agent_ws")
        .and(warp::header::optional::<String>("Authorization"))
        .and(devices_agent)
        .and_then(
            |authorization: Option<String>, devices: Arc<RwLock<Vec<Device>>>| async move {
                if let Some(authorization) = authorization {
                    let host = {
                        let devices_read = &devices.read().await;
                        devices_read
                            .iter()
                            .find(|dev| {
                                dev.api_key
                                    .as_ref()
                                    .is_some_and(|key| key == &authorization)
                            })
                            .and_then(|dev| Some(dev.host.clone()))
                    };
                    if let Some(agent_host) = host {
                        return Ok((agent_host, devices));
                    }
                }
                Err(warp::reject::custom(ApiErrors::NotAuthorized(
                    "not authorized".to_string(),
                )))
            },
        )
        .and(warp::ws())
        .and(agents_warp)
        .map(
            |(agent_host, _devices_agent): (String, Devices), ws: warp::ws::Ws, agent_agents| {
                // This will call our function if the handshake succeeds.
                ws.on_upgrade(move |socket| agent::connected(agent_host, socket, agent_agents))
            },
        );

    let health_check = warp::path("health-check").map(|| format!("Server OK"));
    let public = warp::path("login")
        .and(warp::fs::file("public/index.html"))
        .or(warp::get().and(warp::fs::dir("public/")));
    let routes = health_check
        .or(api.or(gateway.or(agent_gateway.or(public))))
        .recover(handle_rejection);

    let mut monitor_stop = stop_rx.clone();
    let background_task = task::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                _ = monitor_stop.changed() => break,
                _ = interval.tick() => {}
            }
            let user_count = back_users.read().await.len();
            if user_count > 0 || mqtt_enabled {
                let device_list = back_devices.read().await.clone();
                let data = tokio::select! {
                    biased;
                    _ = monitor_stop.changed() => break,
                    data = client::ping_hosts(&device_list, &back_agents) => data,
                };
                if mqtt_enabled {
                    snapshots_tx.send_replace(Some(mqtt::Snapshot {
                        devices: device_list,
                        states: data.clone(),
                    }));
                }
                client::broadcast(client::WsResponse::Update { data }, &back_users).await;
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

    let signal_task = task::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown requested");
        stop_tx.send_replace(true);
    });
    let mut server_stop = stop_rx.clone();
    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(SocketAddr::new(host, port), async move {
            let _ = server_stop.changed().await;
        });
    let server_task = task::spawn(server);
    let _ = background_task.await;
    if let Some(mqtt_task) = mqtt_task {
        let _ = mqtt_task.await;
    }
    // WebSocket clients can stay open indefinitely; bound HTTP draining after MQTT stops.
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
    let _ = signal_task.await;
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn handle_rejection(
    err: warp::reject::Rejection,
) -> std::result::Result<impl warp::reply::Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid Body";
    } else if let Some(e) = err.find::<ApiErrors>() {
        match e {
            ApiErrors::NotAuthorized(_error_message) => {
                code = StatusCode::UNAUTHORIZED;
                message = "Action not authorized";
            }
        }
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        // We should have expected this... Just log and say its a 500
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&ApiErrorResult {
        detail: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
