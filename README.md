# Wake on LAN

A self-hosted dashboard to wake computers on your LAN, monitor their status, and suspend or shut them down through an optional agent. Built with Rust and SvelteKit.

![Wake on LAN dashboard](screenshot.png)

## Getting started

Requires Docker and Docker Compose on a Linux host on the same LAN as your devices. Enable Wake-on-LAN on the computers you want to wake.

```sh
git clone https://github.com/thegamerx1/wakeonlan.git
cd wakeonlan
```

Edit [docker-compose.yml](docker-compose.yml):

- Set `APP_KEY` to a private token.
- Change `APP_PORT` if needed.

Start the app:

```sh
docker compose up -d --build
```

Open `http://<server-ip>:2938` and log in with your `APP_KEY`. Choose **Add device** and enter its name, MAC address, and IP address. The **Host** field requires an IP address; hostnames are not supported.

The container uses host networking. Device settings persist in the Compose `data` volume.

## Suspend and shutdown

Waking a computer needs no agent. To suspend or shut it down, run the agent on that computer (Linux with systemd or Windows).

Edit the device in the dashboard, choose **Regenerate** under **API KEY**, copy the key, and save with **Done**. These browser controls require HTTPS or localhost.

With Rust and Cargo installed, build and run the agent from the repository root:

```sh
cargo build --release -p agent
API_KEY='device-key' URL='ws://<server-ip>:2938/agent_ws' ./target/release/agent
```

## MQTT and Home Assistant

To enable MQTT, create `.env` beside `docker-compose.yml`. Example TLS connection:

```dotenv
MQTT_HOST=mqtt.example.com
MQTT_PORT=8883
MQTT_USERNAME=wakeonlan
MQTT_PASSWORD=your-broker-password
MQTT_TLS=true
```

Recreate the container with `docker compose up -d --build` and connect Home
Assistant's MQTT integration to the same broker. Each computer appears with power,
running status, agent status, and latency entities. OFF requires the agent; set
`SHUTDOWN=true` on the agent to power off instead of suspending.

TLS uses system-trusted certificates. For a private CA, mount its PEM file and add
`MQTT_CA_FILE` to the Compose environment with the certificate's container path.

Example [Mosquitto ACL](https://mosquitto.org/man/mosquitto-conf-5.html) for the
default topics. Add these rules to the file configured by `acl_file`, using your
actual broker usernames, and reload Mosquitto:

```text
user wakeonlan
topic write wakeonlan/#
topic read wakeonlan/+/+/set
topic write homeassistant/device/+/config
topic read homeassistant/status

user homeassistant
topic read wakeonlan/#
topic write wakeonlan/+/+/set
topic readwrite homeassistant/#
```
