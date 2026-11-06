use super::*;
use crate::agent::AgentData;
use crate::client::DeviceData;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::timeout;

fn test_config() -> Config {
    Config {
        options: MqttOptions::new("woltest", "localhost", 1883),
        base: "wakeonlan/woltest".into(),
        discovery_prefix: "homeassistant".into(),
        birth_topic: "homeassistant/status".into(),
        birth_payload: "online".into(),
        retain_discovery: false,
    }
}

fn device() -> Device {
    Device {
        name: "PC \"Office\"\n\\".into(),
        mac: "AA:BB:CC:DD:EE:FF".into(),
        host: "192.0.2.10".into(),
        api_key: Some("secret-never-published".into()),
    }
}

#[test]
fn identity_survives_rename_address_and_mac_format_changes() {
    let config = test_config();
    let original = device();
    let mut renamed = original.clone();
    renamed.name = "New name".into();
    renamed.host = "192.0.2.11".into();
    renamed.mac = "aa-bb-cc-dd-ee-ff".into();
    assert_eq!(config.device_id(&original), config.device_id(&renamed));
    let mut another_installation = test_config();
    another_installation.options = MqttOptions::new("anotherhost", "localhost", 1883);
    assert_ne!(
        config.device_id(&original),
        another_installation.device_id(&original)
    );
    renamed.mac = "bad-mac".into();
    assert!(config.device_id(&renamed).is_none());
    renamed.mac = "02:00:00:00:00:01".into();
    assert_eq!(config.device_id(&renamed).unwrap(), "woltest_020000000001");
}

#[test]
fn discovery_is_valid_json_with_stable_components_and_no_credentials() {
    let config = test_config();
    let device = device();
    let id = config.device_id(&device).unwrap();
    let payload = config.discovery(&device, &id);
    let discovery: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(discovery["device"]["name"], device.name);
    assert_eq!(discovery["device"]["identifiers"][0], id);
    assert_eq!(discovery["origin"]["name"], "wakeonlan");
    for (name, component) in discovery["components"].as_object().unwrap() {
        assert!(component["platform"].is_string());
        assert_eq!(component["unique_id"], format!("{id}_{name}"));
    }
    assert_eq!(discovery["components"]["power"]["retain"], false);
    assert_eq!(discovery["components"]["power"]["optimistic"], false);
    assert!(!payload.contains("secret-never-published"));
}

#[test]
fn power_and_latency_come_from_monitoring() {
    let config = test_config();
    let device = device();
    let state_topic = config.state_topic(&config.device_id(&device).unwrap());
    let mut snapshot = Snapshot {
        devices: vec![device.clone()],
        states: HashMap::new(),
    };
    for (latency, connected, power) in [
        (None, false, "OFF"),
        (Some(12), false, "ON"),
        (None, true, "ON"),
    ] {
        snapshot
            .states
            .insert(device.host.clone(), DeviceData { latency, connected });
        let desired = config.desired(&snapshot);
        let publication = &desired[&state_topic];
        assert!(publication.retain);
        let state: serde_json::Value = serde_json::from_str(&publication.payload).unwrap();
        assert_eq!(state["power"], power);
        assert_eq!(state["agent_connected"], connected);
        assert_eq!(state["latency_ms"], json!(latency));
    }
}

#[test]
fn commands_require_exact_topic_payload_and_no_retain() {
    let config = test_config();
    let topic = config.command_topic("woltest_aabbccddeeff");
    assert!(matches!(
        parse_command(&config, &topic, b"ON", false),
        Some((_, Power::On))
    ));
    assert!(matches!(
        parse_command(&config, &topic, b"OFF", false),
        Some((_, Power::Off))
    ));
    for payload in [b"off".as_slice(), b"garbage", b"ON\n", b"", b"true"] {
        assert!(parse_command(&config, &topic, payload, false).is_none());
    }
    assert!(parse_command(&config, &topic, b"OFF", true).is_none());
    for topic in [
        "other/woltest_aabbccddeeff/set",
        "wakeonlan/woltest//set",
        "wakeonlan/woltest/a/b/set",
        "wakeonlan/woltest/a/state",
    ] {
        assert!(parse_command(&config, topic, b"OFF", false).is_none());
    }
}

#[test]
fn reconciliation_cleans_old_topics_before_discovery_state_and_online() {
    let config = test_config();
    let mut session = Session::new();
    session.desired = config.desired(&Snapshot {
        devices: vec![device()],
        states: HashMap::new(),
    });
    let mut registry = Registry::default();
    registry.topics.insert("old/discovery/config".into());
    let (topic, publication) = session.next(&config, &registry).unwrap();
    assert_eq!(topic, "old/discovery/config");
    assert!(publication.payload.is_empty() && publication.retain);
    registry.topics.clear();
    let (topic, discovery) = session.next(&config, &registry).unwrap();
    assert!(discovery.discovery);
    // An attempted publish does not enter delivered; only the PUBACK path does that.
    assert_eq!(session.next(&config, &registry).unwrap().0, topic);
    session.delivered.insert(topic, discovery);
    let (topic, state) = session.next(&config, &registry).unwrap();
    assert!(topic.ends_with("/state"));
    session.delivered.insert(topic, state);
    let (topic, online) = session.next(&config, &registry).unwrap();
    assert_eq!(topic, config.availability());
    session.delivered.insert(topic, online);
    assert!(session.next(&config, &registry).is_none());
}

#[test]
fn changing_to_non_retained_discovery_clears_previous_retained_config() {
    let config = test_config();
    let mut session = Session::new();
    session.desired = config.desired(&Snapshot {
        devices: vec![device()],
        states: HashMap::new(),
    });
    let topic = config.discovery_topic(&config.device_id(&device()).unwrap());
    let registry = Registry {
        topics: session.desired.keys().cloned().collect(),
        retained_discovery: BTreeSet::from([topic.clone()]),
    };
    let (next_topic, publication) = session.next(&config, &registry).unwrap();
    assert_eq!(next_topic, topic);
    assert!(publication.retain && publication.payload.is_empty());
}

#[tokio::test]
async fn duplicate_expired_and_removed_device_commands_do_not_reach_agent() {
    let config = Arc::new(test_config());
    let device = device();
    let id = config.device_id(&device).unwrap();
    let devices: Devices = Arc::new(RwLock::new(vec![device.clone()]));
    let agents = Agents::default();
    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel();
    agents
        .write()
        .await
        .insert(device.host.clone(), AgentData { tx: agent_tx });
    let (tx, rx) = mpsc::channel(16);
    let (stop_tx, stop_rx) = watch::channel(false);
    let worker = tokio::spawn(commands(config, devices.clone(), agents, rx, stop_rx));
    tx.send(Command {
        id: id.clone(),
        power: Power::Off,
        received: Instant::now() - Duration::from_secs(6),
    })
    .await
    .unwrap();
    tx.send(Command {
        id: "unknown".into(),
        power: Power::Off,
        received: Instant::now(),
    })
    .await
    .unwrap();
    tx.send(Command {
        id: id.clone(),
        power: Power::Off,
        received: Instant::now(),
    })
    .await
    .unwrap();
    let message = timeout(Duration::from_secs(1), agent_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(message.to_str().unwrap(), "suspend");
    tx.send(Command {
        id: id.clone(),
        power: Power::Off,
        received: Instant::now(),
    })
    .await
    .unwrap();
    assert!(timeout(Duration::from_millis(100), agent_rx.recv())
        .await
        .is_err());
    devices.write().await.clear();
    tx.send(Command {
        id,
        power: Power::Off,
        received: Instant::now(),
    })
    .await
    .unwrap();
    assert!(timeout(Duration::from_millis(100), agent_rx.recv())
        .await
        .is_err());
    stop_tx.send_replace(true);
    worker.await.unwrap();
}
