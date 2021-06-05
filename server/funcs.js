const db = require("./initdb")
const wol = require("wakeonlan")
const pong = require("./ping").ping

var pingCache = {}
const pingInterval = 5 * 1000

const needed = ["name", "mac", "host"]
var wss
refresh()

setInterval(refresh, pingInterval)

function refresh() {
	return new Promise(resolve => {
		const promises = []
		const outnew = {}
		db.get("devices").forEach(device => {
			promises.push(
				pong(device.host).then(status => {
					outnew[device.host] = status
				})
			)
		})
		Promise.all(promises).then(() => {
			pingCache = outnew
			wss.broadcast({ event: "update", data: outnew })
			resolve()
		})
	})
}

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
			ws.Send({ event: "wake", success: true, nonce: data.nonce })
		})
		.catch(() => {
			ws.Send({ event: "wake", success: false, nonce: data.nonce })
		})
}

function save(data, ws) {
	if (notAuth(ws)) return
	const out = Array.isArray(data.devices) ? data.devices : []
	let error = false
	out.forEach((e, i) => {
		needed.forEach(check => {
			if (typeof e[check] !== "string" || e[check] === "") {
				out.splice(i, 1)
				error = true
			} else if (e[check].length > 18) {
				out[i][check] = e[check].substr(0, 18)
			}
		})
	})
	db.set("devices", out)
	db.sync()
	ws.Send({ event: "save", success: !error, nonce: data.nonce })
	const devices = db.get("devices")
	wss.broadcast({ event: "devices", devices })
	refresh()
}

function login(data, ws) {
	if (ws.authenticated) return
	const success = data.login === process.env.APP_KEY
	console.log(data.login, process.env.APP_KEY)
	if (success) ws.authenticated = true
	ws.Send({
		event: "login",
		success: success,
		devices: success ? db.get("devices") : null,
		onlines: pingCache,
		nonce: data.nonce,
	})
}

function init(ws) {
	wss = ws
}

module.exports = { wake, save, login, init }
