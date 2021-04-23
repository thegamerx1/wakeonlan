const db = require("./initdb")
const wol = require("wakeonlan")
const pong = require("./ping").ping

const needed = ["name", "mac", "host"]

function notAuth(ws) {
	if (!ws.authenticated) {
		ws.terminate()
		return true
	}
}

function wake(data, ws) {
	if (notAuth(ws)) return
	wol(data.mac)
		.then(() => {
			wsSend(ws, { event: "wake", success: true, nonce: data.nonce })
		})
		.catch(() => {
			wsSend(ws, { event: "wake", success: false, nonce: data.nonce })
		})
}

function ping(data, ws) {
	if (notAuth(ws)) return
	pong(data.host).then(status => {
		wsSend(ws, { event: "ping", status, nonce: data.nonce })
	})
}

function save(data, ws) {
	if (notAuth(ws)) return
	const out = Array.isArray(data.devices) ? data.devices : []
	out.forEach((e, i) => {
		needed.forEach(check => {
			if (typeof e[check] !== "string" || e[check] === "") {
				out.splice(i, 1)
			}
		})
	})
	db.set("devices", out)
	db.sync()
	wsSend(ws, { event: "save", devices: out, nonce: data.nonce })
}

function login(data, ws) {
	if (ws.authenticated) return
	const success = data.login === process.env.key
	if (success) ws.authenticated = true
	wsSend(ws, {
		event: "login",
		success: success,
		devices: success ? db.get("devices") : null,
		nonce: data.nonce,
	})
}

function wsSend(ws, obj) {
	if (ws.readyState === ws.OPEN) {
		ws.send(JSON.stringify(obj))
	}
}

module.exports = { ping, wake, save, login }
