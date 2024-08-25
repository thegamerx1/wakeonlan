#[macro_use]
extern crate log;

use std::{env, io::Error, process::Command, time::Duration};

use http::{HeaderValue, Request};
use tungstenite::{client::IntoClientRequest, connect, Message};

fn create_req(url: &str, api_key: &str) -> Request<()> {
    let mut req = url.into_client_request().unwrap();
    let headers = req.headers_mut();
    headers.append("Authorization", HeaderValue::from_str(api_key).unwrap());

    req
}

fn main() {
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
        let (mut socket, _response) = match connect(req.clone()) {
            Ok(soc) => soc,
            Err(err) => {
                warn!("Error connecting {err}");
                continue;
            }
        };

        info!("Connected to the server");

        socket.send(Message::Text("hey".into())).unwrap();
        loop {
            let msg = match socket.read() {
                Ok(msg) => msg,
                Err(err) => {
                    warn!("Error sending message {err}");
                    break;
                }
            };
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
                    socket.send(Message::Pong(p)).ok();
                }
                _ => {
                    warn!("Received unknown message {msg}");
                }
            }
        }
        socket.close(None).ok();
    }
    // socket.close(None);
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
