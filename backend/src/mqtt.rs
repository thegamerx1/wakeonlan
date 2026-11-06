//! One connection owner, one acknowledged publication in flight, and coalesced snapshots.
use crate::agent::Agents;
use crate::client::{send_shutdown, Device, Devices, UpdateDevices};
use rand::Rng;
use rumqttc::{
    AsyncClient, Event, EventLoop, LastWill, MqttOptions, Outgoing, Packet, QoS, SubscribeFilter,
    SubscribeReasonCode, TlsConfiguration, Transport,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time::{interval, sleep, Instant, MissedTickBehavior};
use wol::{send_wol, MacAddr};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const REGISTRY_PATH: &str = "data/mqtt-topics.json";
const ACK_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Default)]
pub struct Snapshot {
    pub devices: Vec<Device>,
    pub states: UpdateDevices,
}

pub struct Config {
    options: MqttOptions,
    base: String,
    discovery_prefix: String,
    birth_topic: String,
    birth_payload: String,
    retain_discovery: bool,
}

fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn boolean(name: &str, default: bool) -> Result<bool> {
    match env::var(name).as_deref() {
        Err(env::VarError::NotPresent) => Ok(default),
        Ok("true" | "1") => Ok(true),
        Ok("false" | "0") => Ok(false),
        _ => Err(format!("{name} must be true or false").into()),
    }
}

fn topic(value: String, name: &str) -> Result<String> {
    if value.is_empty()
        || value.len() > 512
        || value.contains(['+', '#', '\0'])
        || value.starts_with('/')
        || value.ends_with('/')
    {
        return Err(format!("{name} must be a nonempty MQTT topic without wildcards").into());
    }
    Ok(value)
}

fn tls_connector(ca_bundle: Option<&[u8]>) -> Result<native_tls::TlsConnector> {
    let mut builder = native_tls::TlsConnector::builder();
    builder.min_protocol_version(Some(native_tls::Protocol::Tlsv12));
    if let Some(pem) = ca_bundle {
        let certificates = native_tls::Certificate::stack_from_pem(pem)?;
        if certificates.is_empty() {
            return Err("MQTT_CA_FILE must contain at least one valid PEM certificate".into());
        }
        // A supplied CA bundle is authoritative, matching the previous MQTT TLS behavior.
        builder.disable_built_in_roots(true);
        for certificate in certificates {
            builder.add_root_certificate(certificate);
        }
    }
    Ok(builder.build()?)
}

impl Config {
    pub async fn from_env() -> Result<Option<Self>> {
        let host = match env::var("MQTT_HOST") {
            Err(env::VarError::NotPresent) => return Ok(None),
            Err(_) => return Err("MQTT_HOST must contain valid UTF-8".into()),
            Ok(host) if !host.is_empty() && !host.contains(['/', '@']) => host,
            _ => {
                return Err(
                    "MQTT_HOST must be a hostname or IP address, without a URL scheme".into(),
                )
            }
        };
        let tls = boolean("MQTT_TLS", false)?;
        let port = setting("MQTT_PORT", if tls { "8883" } else { "1883" })
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or("MQTT_PORT must be between 1 and 65535")?;
        let client_id = match env::var("MQTT_CLIENT_ID") {
            Ok(id) => id,
            Err(_) => match tokio::fs::read_to_string("data/mqtt-client-id").await {
                Ok(id) => id.trim().to_owned(),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    let id = format!("wol{:016x}", rand::random::<u64>());
                    tokio::fs::write("data/mqtt-client-id", &id).await?;
                    id
                }
                Err(err) => return Err(err.into()),
            },
        };
        if client_id.is_empty()
            || client_id.len() > 64
            || !client_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(
                "MQTT_CLIENT_ID must contain 1-64 letters, digits, underscores or hyphens".into(),
            );
        }
        let base = topic(
            setting("MQTT_BASE_TOPIC", &format!("wakeonlan/{client_id}")),
            "MQTT_BASE_TOPIC",
        )?;
        let discovery_prefix = topic(
            setting("MQTT_DISCOVERY_PREFIX", "homeassistant"),
            "MQTT_DISCOVERY_PREFIX",
        )?;
        let birth_topic = topic(
            setting(
                "MQTT_HA_STATUS_TOPIC",
                &format!("{discovery_prefix}/status"),
            ),
            "MQTT_HA_STATUS_TOPIC",
        )?;
        let birth_payload = setting("MQTT_HA_ONLINE_PAYLOAD", "online");
        if birth_payload.is_empty() {
            return Err("MQTT_HA_ONLINE_PAYLOAD cannot be empty".into());
        }
        let mut options = MqttOptions::new(&client_id, host, port);
        // Clean MQTT 3.1.1 sessions deliberately prevent replay of offline commands.
        options.set_clean_session(true);
        options.set_keep_alive(Duration::from_secs(60));
        options.set_inflight(1);
        options.set_max_packet_size(16 * 1024, 64 * 1024);
        options.set_last_will(LastWill::new(
            format!("{base}/availability"),
            "offline",
            QoS::AtLeastOnce,
            true,
        ));
        if let Ok(username) = env::var("MQTT_USERNAME") {
            options.set_credentials(username, setting("MQTT_PASSWORD", ""));
        } else if env::var_os("MQTT_PASSWORD").is_some() {
            return Err("MQTT_PASSWORD requires MQTT_USERNAME".into());
        }
        if let Ok(ca_path) = env::var("MQTT_CA_FILE") {
            if !tls {
                return Err("MQTT_CA_FILE requires MQTT_TLS=true".into());
            }
            let ca_bundle = tokio::fs::read(ca_path).await?;
            options.set_transport(Transport::tls_with_config(
                TlsConfiguration::NativeConnector(tls_connector(Some(&ca_bundle))?),
            ));
        } else if tls {
            options.set_transport(Transport::tls_with_config(
                TlsConfiguration::NativeConnector(tls_connector(None)?),
            ));
        }
        Ok(Some(Self {
            options,
            base,
            discovery_prefix,
            birth_topic,
            birth_payload,
            retain_discovery: boolean("MQTT_DISCOVERY_RETAIN", false)?,
        }))
    }

    fn availability(&self) -> String {
        format!("{}/availability", self.base)
    }

    fn device_id(&self, device: &Device) -> Option<String> {
        // Normalize MAC formatting; display name and IP are deliberately not identity.
        let mac = MacAddr::from_str(&device.mac).ok()?;
        let mac: String = mac.0.iter().map(|byte| format!("{byte:02x}")).collect();
        Some(format!("{}_{}", self.options.client_id(), mac))
    }

    fn discovery_topic(&self, id: &str) -> String {
        format!("{}/device/{id}/config", self.discovery_prefix)
    }

    fn state_topic(&self, id: &str) -> String {
        format!("{}/{id}/state", self.base)
    }

    fn command_topic(&self, id: &str) -> String {
        format!("{}/{id}/set", self.base)
    }

    fn discovery(&self, device: &Device, id: &str) -> String {
        let state_topic = self.state_topic(id);
        json!({
            "device": {
                "identifiers": [id], "name": device.name,
                "manufacturer": "Wake on LAN", "model": "Computer"
            },
            "origin": {"name": "wakeonlan", "sw_version": env!("CARGO_PKG_VERSION")},
            "availability_topic": self.availability(),
            "payload_available": "online", "payload_not_available": "offline",
            "qos": 1,
            "components": {
                "power": {
                    "platform": "switch", "unique_id": format!("{id}_power"),
                    "name": null, "icon": "mdi:desktop-tower",
                    "command_topic": self.command_topic(id), "state_topic": state_topic,
                    "value_template": "{{ value_json.power }}",
                    "payload_on": "ON", "payload_off": "OFF", "retain": false,
                    "optimistic": false,
                    "json_attributes_topic": state_topic
                },
                "running": {
                    "platform": "binary_sensor", "unique_id": format!("{id}_running"),
                    "name": "Running", "device_class": "running",
                    "state_topic": state_topic, "value_template": "{{ value_json.power }}",
                    "payload_on": "ON", "payload_off": "OFF"
                },
                "agent": {
                    "platform": "binary_sensor", "unique_id": format!("{id}_agent"),
                    "name": "Agent", "device_class": "connectivity", "entity_category": "diagnostic",
                    "state_topic": state_topic,
                    "value_template": "{{ 'ON' if value_json.agent_connected else 'OFF' }}",
                    "payload_on": "ON", "payload_off": "OFF"
                },
                "latency": {
                    "platform": "sensor", "unique_id": format!("{id}_latency"),
                    "name": "Latency", "unit_of_measurement": "ms", "state_class": "measurement",
                    "entity_category": "diagnostic", "state_topic": state_topic,
                    "value_template": "{{ value_json.latency_ms if value_json.latency_ms is not none else 'None' }}"
                }
            }
        }).to_string()
    }

    fn desired(&self, snapshot: &Snapshot) -> BTreeMap<String, Publication> {
        let mut desired = BTreeMap::new();
        for device in &snapshot.devices {
            let Some(id) = self.device_id(device) else {
                warn!("MQTT: skipping device with invalid MAC address");
                continue;
            };
            desired.insert(
                self.discovery_topic(&id),
                Publication {
                    payload: self.discovery(device, &id),
                    retain: self.retain_discovery,
                    discovery: true,
                },
            );
            let state = snapshot.states.get(&device.host);
            let agent_connected = state.is_some_and(|s| s.connected);
            let latency = state.and_then(|s| s.latency);
            desired.insert(
                self.state_topic(&id),
                Publication {
                    payload: json!({
                        "power": if agent_connected || latency.is_some() { "ON" } else { "OFF" },
                        "agent_connected": agent_connected, "latency_ms": latency,
                        "host": device.host, "mac": device.mac
                    })
                    .to_string(),
                    retain: true,
                    discovery: false,
                },
            );
        }
        desired
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Publication {
    payload: String,
    retain: bool,
    discovery: bool,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct Registry {
    topics: BTreeSet<String>,
    retained_discovery: BTreeSet<String>,
}

impl Registry {
    async fn claim(&mut self, topic: &str, retained_discovery: bool) -> Result<()> {
        if self.topics.contains(topic)
            && (!retained_discovery || self.retained_discovery.contains(topic))
        {
            return Ok(());
        }
        let mut next = self.clone();
        next.topics.insert(topic.to_owned());
        if retained_discovery {
            next.retained_discovery.insert(topic.to_owned());
        }
        save_registry(&next).await?;
        *self = next;
        Ok(())
    }

    async fn forget(&mut self, topic: &str) -> Result<()> {
        let mut next = self.clone();
        next.topics.remove(topic);
        next.retained_discovery.remove(topic);
        save_registry(&next).await?;
        *self = next;
        Ok(())
    }
}

async fn save_registry(owned: &Registry) -> Result<()> {
    let temporary = format!("{REGISTRY_PATH}.tmp");
    tokio::fs::write(&temporary, serde_json::to_vec_pretty(owned)?).await?;
    tokio::fs::rename(temporary, REGISTRY_PATH).await?;
    Ok(())
}

enum Operation {
    Subscribe,
    Publish(String, Publication),
    Unavailable,
    Offline,
    Disconnect,
}

struct Pending {
    operation: Operation,
    packet_id: Option<u16>,
    since: Instant,
}

struct Session {
    connected: bool,
    subscribed: bool,
    ready: bool,
    stopping: Option<Instant>,
    pending: Option<Pending>,
    delivered: BTreeMap<String, Publication>,
    desired: BTreeMap<String, Publication>,
    initialized: bool,
    offline_published: bool,
    birth_at: Option<Instant>,
}

impl Session {
    fn new() -> Self {
        Self {
            connected: false,
            subscribed: false,
            ready: false,
            stopping: None,
            pending: None,
            delivered: BTreeMap::new(),
            desired: BTreeMap::new(),
            initialized: false,
            offline_published: false,
            birth_at: None,
        }
    }

    // Discovery precedes state, and functional availability is announced last.
    fn next(&self, config: &Config, owned: &Registry) -> Option<(String, Publication)> {
        if let Some(stale) = owned
            .topics
            .iter()
            .find(|t| !self.desired.contains_key(*t) && **t != config.availability())
        {
            return Some((
                stale.clone(),
                Publication {
                    payload: String::new(),
                    retain: true,
                    discovery: false,
                },
            ));
        }
        // Switching retained discovery off must first clear the broker's previous config.
        if let Some(previous) = owned
            .retained_discovery
            .iter()
            .find(|t| self.desired.get(*t).is_some_and(|p| !p.retain))
        {
            return Some((
                previous.clone(),
                Publication {
                    payload: String::new(),
                    retain: true,
                    discovery: false,
                },
            ));
        }
        for discovery in [true, false] {
            if let Some((topic, publication)) = self
                .desired
                .iter()
                .find(|(t, p)| p.discovery == discovery && self.delivered.get(*t) != Some(*p))
            {
                return Some((topic.clone(), publication.clone()));
            }
        }
        let availability = config.availability();
        if !self.delivered.contains_key(&availability) {
            return Some((
                availability,
                Publication {
                    payload: "online".into(),
                    retain: true,
                    discovery: false,
                },
            ));
        }
        None
    }

    async fn drive(
        &mut self,
        config: &Config,
        client: &AsyncClient,
        owned: &mut Registry,
    ) -> Result<()> {
        if self
            .stopping
            .is_some_and(|since| since.elapsed() > Duration::from_secs(5))
        {
            return Err("MQTT graceful shutdown timed out; leaving LWT to mark offline".into());
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.since.elapsed() > ACK_TIMEOUT)
        {
            return Err("MQTT acknowledgement timed out".into());
        }
        if !self.connected || self.pending.is_some() {
            return Ok(());
        }
        if self.stopping.is_some() {
            client.try_publish(config.availability(), QoS::AtLeastOnce, true, "offline")?;
            self.pending = Some(Pending {
                operation: Operation::Offline,
                packet_id: None,
                since: Instant::now(),
            });
            return Ok(());
        }
        if !self.subscribed {
            return Ok(());
        }
        if !self.offline_published {
            owned.claim(&config.availability(), false).await?;
            client.try_publish(config.availability(), QoS::AtLeastOnce, true, "offline")?;
            self.pending = Some(Pending {
                operation: Operation::Unavailable,
                packet_id: None,
                since: Instant::now(),
            });
            return Ok(());
        }
        if !self.initialized {
            return Ok(());
        }
        if self.birth_at.is_some_and(|at| Instant::now() >= at) {
            self.birth_at = None;
            if !config.retain_discovery {
                self.delivered.clear();
            }
            self.delivered.remove(&config.availability());
        }
        if let Some((topic, publication)) = self.next(config, owned) {
            // Record ownership before publishing, so a crash cannot orphan retained data.
            owned
                .claim(&topic, publication.discovery && publication.retain)
                .await?;
            client.try_publish(
                &topic,
                QoS::AtLeastOnce,
                publication.retain,
                publication.payload.as_bytes(),
            )?;
            self.pending = Some(Pending {
                operation: Operation::Publish(topic, publication),
                packet_id: None,
                since: Instant::now(),
            });
        } else if !self.ready {
            self.ready = true;
            info!(
                "MQTT ready: client_id={}, devices={}",
                config.options.client_id(),
                self.desired.len() / 2
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Power {
    On,
    Off,
}

fn parse_command(
    config: &Config,
    topic: &str,
    payload: &[u8],
    retained: bool,
) -> Option<(String, Power)> {
    if retained {
        return None;
    }
    let id = topic
        .strip_prefix(&format!("{}/", config.base))?
        .strip_suffix("/set")?;
    if id.is_empty() || id.contains('/') {
        return None;
    }
    let power = match payload {
        b"ON" => Power::On,
        b"OFF" => Power::Off,
        _ => return None,
    };
    Some((id.to_owned(), power))
}

struct Command {
    id: String,
    power: Power,
    received: Instant,
}

async fn commands(
    config: std::sync::Arc<Config>,
    devices: Devices,
    agents: Agents,
    mut rx: mpsc::Receiver<Command>,
    mut stop: watch::Receiver<bool>,
) {
    let mut recent: HashMap<String, (Power, Instant)> = HashMap::new();
    loop {
        let command = tokio::select! {
            biased;
            _ = stop.changed() => break,
            command = rx.recv() => match command { Some(c) => c, None => break },
        };
        if *stop.borrow() || command.received.elapsed() > Duration::from_secs(5) {
            continue;
        }
        let devices = devices.read().await.clone();
        recent.retain(|id, _| {
            devices
                .iter()
                .any(|d| config.device_id(d).as_ref() == Some(id))
        });
        let Some(device) = devices
            .iter()
            .find(|d| config.device_id(d).as_ref() == Some(&command.id))
        else {
            continue;
        };
        // QoS 1 retransmissions must not repeatedly suspend a computer that wakes again.
        if recent.get(&command.id).is_some_and(|(power, at)| {
            *power == command.power && at.elapsed() < Duration::from_secs(30)
        }) {
            continue;
        }
        let success = match command.power {
            Power::On => match MacAddr::from_str(&device.mac) {
                Ok(mac) => send_wol(mac, None, None).is_ok(),
                Err(_) => false,
            },
            Power::Off => send_shutdown(&agents, &device.host).await,
        };
        if success {
            recent.insert(command.id.clone(), (command.power, Instant::now()));
            info!(
                "MQTT command accepted: device_id={}, power={}",
                command.id,
                if command.power == Power::On {
                    "ON"
                } else {
                    "OFF"
                }
            );
        } else {
            warn!(
                "MQTT command failed: device_id={} (OFF requires a connected agent)",
                command.id
            );
        }
        // Only the monitoring loop publishes actual power state, never an optimistic command result.
    }
}

struct Connection {
    client: AsyncClient,
    events: EventLoop,
}

async fn connected_session(
    config: &Config,
    connection: &mut Connection,
    snapshots: &mut watch::Receiver<Option<Snapshot>>,
    stop: &mut watch::Receiver<bool>,
    owned: &mut Registry,
    commands: &mpsc::Sender<Command>,
    stable: &mut bool,
) -> Result<()> {
    let Connection { client, events } = connection;
    let mut session = Session::new();
    if let Some(snapshot) = snapshots.borrow_and_update().as_ref() {
        session.desired = config.desired(snapshot);
        session.initialized = true;
    }
    let mut tick = interval(Duration::from_millis(25));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut connected_at = None;
    loop {
        // Keep each poll alive across timer/snapshot events, including slow DNS/TLS handshakes.
        let event = {
            let poll = events.poll();
            tokio::pin!(poll);
            loop {
                tokio::select! {
                    biased;
                    _ = stop.changed(), if session.stopping.is_none() => {
                        if !session.connected { return Ok(()); }
                        session.ready = false;
                        session.stopping = Some(Instant::now());
                    }
                    result = &mut poll => break result?,
                    _ = tick.tick() => {
                        if connected_at.is_some_and(|at: Instant| at.elapsed() >= Duration::from_secs(60)) { *stable = true; }
                        session.drive(config, client, owned).await?;
                    }
                    changed = snapshots.changed(), if session.stopping.is_none() => {
                        changed?;
                        if let Some(snapshot) = snapshots.borrow_and_update().as_ref() {
                            session.desired = config.desired(snapshot);
                            session.initialized = true;
                        }
                    }
                }
            }
        };
        match event {
            Event::Incoming(Packet::ConnAck(ack)) => {
                session.connected = true;
                connected_at = Some(Instant::now());
                info!(
                    "MQTT connected: client_id={}, session_present={}",
                    config.options.client_id(),
                    ack.session_present
                );
                client.try_subscribe_many([
                    SubscribeFilter::new(config.birth_topic.clone(), QoS::AtLeastOnce),
                    SubscribeFilter::new(format!("{}/+/set", config.base), QoS::AtLeastOnce),
                ])?;
                session.pending = Some(Pending {
                    operation: Operation::Subscribe,
                    packet_id: None,
                    since: Instant::now(),
                });
            }
            Event::Outgoing(Outgoing::Subscribe(id) | Outgoing::Publish(id)) => {
                if let Some(pending) = &mut session.pending {
                    pending.packet_id = Some(id);
                }
            }
            Event::Incoming(Packet::SubAck(ack)) => {
                if session.pending.as_ref().is_some_and(|p| {
                    matches!(p.operation, Operation::Subscribe) && p.packet_id == Some(ack.pkid)
                }) {
                    if ack.return_codes.len() != 2
                        || ack
                            .return_codes
                            .iter()
                            .any(|code| matches!(code, SubscribeReasonCode::Failure))
                    {
                        return Err("MQTT subscription denied by broker; check ACLs".into());
                    }
                    session.subscribed = true;
                    session.pending = None;
                    debug!("MQTT command and HA birth subscriptions acknowledged");
                }
            }
            Event::Incoming(Packet::PubAck(ack)) => {
                if session
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.packet_id == Some(ack.pkid))
                {
                    let pending = session.pending.take().unwrap();
                    match pending.operation {
                        Operation::Publish(topic, publication) => {
                            if publication.payload.is_empty() {
                                owned.forget(&topic).await?;
                                session.delivered.remove(&topic);
                            } else {
                                session.delivered.insert(topic, publication);
                            }
                        }
                        Operation::Offline => {
                            client.try_disconnect()?;
                            session.pending = Some(Pending {
                                operation: Operation::Disconnect,
                                packet_id: None,
                                since: Instant::now(),
                            });
                        }
                        Operation::Unavailable => {
                            session.offline_published = true;
                        }
                        Operation::Subscribe | Operation::Disconnect => {}
                    }
                }
            }
            Event::Incoming(Packet::Publish(message)) if session.stopping.is_none() => {
                if message.topic == config.birth_topic
                    && message.payload.as_ref() == config.birth_payload.as_bytes()
                {
                    if session.birth_at.is_none() {
                        session.birth_at = Some(
                            Instant::now()
                                + Duration::from_millis(rand::thread_rng().gen_range(0..=5000)),
                        );
                        info!("MQTT Home Assistant birth received; reconciliation scheduled");
                    }
                } else if session.subscribed && session.initialized {
                    if let Some((id, power)) =
                        parse_command(config, &message.topic, &message.payload, message.retain)
                    {
                        if commands
                            .try_send(Command {
                                id,
                                power,
                                received: Instant::now(),
                            })
                            .is_err()
                        {
                            warn!("MQTT command queue full; command rejected");
                        }
                    } else {
                        debug!("MQTT ignored invalid or retained command");
                    }
                }
            }
            Event::Outgoing(Outgoing::Disconnect) if session.stopping.is_some() => return Ok(()),
            _ => {}
        }
    }
}

pub async fn run(
    config: Config,
    mut snapshots: watch::Receiver<Option<Snapshot>>,
    devices: Devices,
    agents: Agents,
    mut stop: watch::Receiver<bool>,
) {
    let config = std::sync::Arc::new(config);
    let (client, events) = AsyncClient::new(config.options.clone(), 4);
    let mut connection = Connection { client, events };
    let mut owned: Registry = match tokio::fs::read(REGISTRY_PATH).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(topics) => topics,
            Err(err) => {
                error!("MQTT topic registry is invalid: {err}");
                return;
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Registry::default(),
        Err(err) => {
            error!("MQTT cannot load topic registry: {err}");
            return;
        }
    };
    let (tx, rx) = mpsc::channel(16);
    let worker = tokio::spawn(commands(config.clone(), devices, agents, rx, stop.clone()));
    let (host, port) = config.options.broker_address();
    info!(
        "MQTT starting: client_id={}, broker={host}:{port}, protocol=3.1.1",
        config.options.client_id()
    );
    let mut attempt = 0u32;
    while !*stop.borrow() {
        let mut stable = false;
        let result = connected_session(
            &config,
            &mut connection,
            &mut snapshots,
            &mut stop,
            &mut owned,
            &tx,
            &mut stable,
        )
        .await;
        if *stop.borrow() {
            if let Err(err) = result {
                warn!("MQTT shutdown: {err}");
            }
            break;
        }
        if stable {
            attempt = 0;
        }
        let cap_ms = (1000u64 << attempt.min(6)).min(60_000);
        let mut delay = Duration::from_millis(rand::thread_rng().gen_range(0..=cap_ms));
        if let Err(err) = &result {
            // Authentication/TLS/ACL errors deserve a slower cadence than network loss.
            let reason = err.to_string();
            if [
                "NotAuthorized",
                "BadUserNamePassword",
                "TLS",
                "Tls",
                "certificate",
                "denied",
            ]
            .iter()
            .any(|word| reason.contains(word))
            {
                delay = delay.max(Duration::from_secs(30));
            }
            warn!(
                "MQTT disconnected: client_id={}, reason={err}, retry_in={delay:?}, attempt={}",
                config.options.client_id(),
                attempt + 1
            );
        }
        attempt = attempt.saturating_add(1);
        // Clean sessions + authoritative snapshots: never replay stale queued publications.
        connection.events.clean();
        connection.events.pending.clear();
        connection.events.state = rumqttc::MqttState::new(1, false);
        tokio::select! { biased; _ = stop.changed() => break, _ = sleep(delay) => {} }
    }
    drop(tx);
    let _ = worker.await;
    info!("MQTT stopped: client_id={}", config.options.client_id());
}

#[cfg(test)]
#[path = "mqtt_tests.rs"]
mod tests;
