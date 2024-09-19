#[macro_use]
extern crate log;

use std::{env, io::Error, process::Command, time::Duration};

use async_tungstenite::tungstenite::Message;
use async_tungstenite::{tokio::connect_async, tungstenite::client::IntoClientRequest};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderValue, Request};
use tokio::time::timeout;

const HEARTBEAT: Duration = Duration::from_secs(15);
const TIMEOUT: Duration = Duration::from_secs(20);

fn create_req(url: &str, api_key: &str) -> Request<()> {
    let mut req = url.into_client_request().unwrap();
    let headers = req.headers_mut();
    headers.append("Authorization", HeaderValue::from_str(api_key).unwrap());

    req
}

#[tokio::main]
async fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    pretty_env_logger::init_timed();

    let api_key = env::var("API_KEY").expect("Missing environment variable API_KEY");
    let url = env::var("URL").expect("Missing environment variable URL");

    let req = create_req(&url, &api_key);

    loop {
        info!("Disconnected");
        std::thread::sleep(Duration::from_secs(5));
        info!("Trying to connect");
        let (ws_stream, _response) = match connect_async(req.clone()).await {
            Ok(soc) => soc,
            Err(err) => {
                warn!("Error connecting {err}");
                continue;
            }
        };

        let (mut write, mut read) = ws_stream.split();

        info!("Connected to the server");

        match write.send(Message::Ping(vec![255,69,0,69])).await {
            Ok(()) => (),
            Err(err) => {
                error!("Failed sending connect ping: {err}");
                break;
            }
        };

        loop {
            let msg = match timeout(TIMEOUT, read.next()).await {
                Ok(msg) => msg,
                Err(_elapsed) => break,
            };

            if let Some(Ok(msg)) = msg {
                match msg {
                    Message::Text(msg) => {
                        if msg == "suspend" {
                            warn!("Received suspend");
                            suspend();
                            std::thread::sleep(Duration::from_secs(5));
                            break;
                        } else {
                            warn!("Received unknown message {msg}");
                        }
                    }
                    Message::Pong(_p) => {
                        debug!("Recived pong");
                    }
                    Message::Ping(p) => {
                        debug!("Recived ping replying with pong");
                        write.send(Message::Pong(p)).await.ok();
                    }
                    _ => {
                        warn!("Received unknown message {msg}");
                    }
                }
            }
        }
    }
}

fn suspend() -> Result<bool, Error> {
    let command = if cfg!(windows) {
        ("shutdown", vec!["/s", "/t", "0"])
    } else {
        ("systemctl", vec!["suspend"])
    };

    let output = Command::new(command.0).args(command.1).output()?;

    Ok(output.status.success())
}
