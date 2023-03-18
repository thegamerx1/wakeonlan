import { db } from "./initdb"
// @ts-ignore
// FIXME: This is not cool
import wol from "wakeonlan"
import { ping } from "./ping"
import { Device, LoginEvent, SaveEvent, WakeEvent, WSServer, WSSocket } from "./websocket"

const pingInterval = 5 * 1000
const password = process.env.APP_KEY
if (!password) {
	throw new Error("Invalid password: " + password)
}

var wss: WSServer
refresh()

setInterval(refresh, pingInterval)

interface Outnew {
	[key: string]: number
}

async function refresh() {
	const promises: Promise<number | void>[] = []
	const outnew: Outnew = {}
	db.get("devices").forEach((device: Device) => {
		promises.push(
			ping(device.host).then(status => {
				outnew[device.host] = Math.floor(status)
			})
		)
	})

	await Promise.all(promises)
	wss.broadcast({ event: "update", data: outnew })
}

function wake(data: WakeEvent, ws: WSSocket) {
	wol(data.mac)
		.then(() => {
			ws.Send({ event: "wake", success: true, nonce: data.nonce })
		})
		.catch(() => {
			ws.Send({ event: "wake", success: false, nonce: data.nonce })
		})
}

function save(data: SaveEvent, ws: WSSocket) {
	let error = false
	db.set("devices", data.devices)
	db.sync()
	ws.Send({ event: "save", success: !error, nonce: data.nonce })
	const devices = db.get("devices")
	wss.broadcast({ event: "devices", devices })
	refresh()
}

function login(data: LoginEvent, ws: WSSocket) {
	const success = data.login === password
	if (success) {
		ws.authenticated = true
		refresh()
	}
	ws.Send({
		event: "login",
		success: success,
		devices: success ? db.get("devices") : null,
		nonce: data.nonce,
	})
}

function init(ws: WSServer) {
	wss = ws
}

const funcs = { wake, save, login, init }
export default funcs
