use std::fs::File;
use std::io::{BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;

use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

fn main() {
    // Load the CA certificate
    let mut ca_reader = BufReader::new(File::open("ca.crt").unwrap());
    let ca_cert = rustls::Certificate(certs(&mut ca_reader).unwrap()[0].clone());

    // Load the server certificate and private key
    let mut cert_reader = BufReader::new(File::open("server.crt").unwrap());
    let mut key_reader = BufReader::new(File::open("server.key").unwrap());

    let server_cert = certs(&mut cert_reader).unwrap();
    let server_key = PrivateKey(pkcs8_private_keys(&mut key_reader).unwrap()[0].clone());

    // Create the TLS configuration
    let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13]).unwrap()

    let tls_config = Arc::new(config);

    // Create a TCP listener
    let listener = TcpListener::bind("0.0.0.0:8443").unwrap();
    println!("Server listening on port 8443");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tls_config = Arc::clone(&tls_config);
                std::thread::spawn(move || {
                    let mut tls_stream = rustls::ServerConnection::new(tls_config).unwrap();
                    let mut stream = rustls::Stream::new(&mut tls_stream, &mut stream);

                    // Handle the connection
                    let response = "Hello, TLS Client!";
                    stream.write_all(response.as_bytes()).unwrap();
                });
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}