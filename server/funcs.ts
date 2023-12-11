import { db } from "./initdb"
// @ts-ignore
// FIXME: This is not cool
import wol from "wakeonlan"
import { ping } from "./ping"
import { Device, LoginEvent, SaveEvent, WakeEvent, WSServer, WSSocket } from "./websocket"

const pingInterval = 5 * 1000
const PASSWORD = process.env.APP_KEY

if (!PASSWORD) {
	throw new Error("Invalid password: " + PASSWORD)
}

interface Outnew {
	[key: string]: number
}

class EventManager {
	wss: WSServer

	constructor(ws: WSServer) {
		this.wss = ws
		setInterval(this.refresh.bind(this), pingInterval)
	}

	async refresh() {
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
		this.wss.broadcast({ event: "update", data: outnew })
	}

	wake(data: WakeEvent, ws: WSSocket) {
		wol(data.mac)
			.then(() => {
				ws.Send({ event: "wake", success: true, nonce: data.nonce })
			})
			.catch(() => {
				ws.Send({ event: "wake", success: false, nonce: data.nonce })
			})
	}

	save(data: SaveEvent, ws: WSSocket) {
		let error = false
		db.set("devices", data.devices)
		db.sync()
		ws.Send({ event: "save", success: !error, nonce: data.nonce })
		const devices = db.get("devices")
		this.wss.broadcast({ event: "devices", devices })
		this.refresh()
	}

	login(data: LoginEvent, ws: WSSocket) {
		const success = data.login === PASSWORD
		if (success) {
			ws.authenticated = true
			this.refresh()
		}
		ws.Send({
			event: "login",
			success: success,
			devices: success ? db.get("devices") : null,
			nonce: data.nonce,
		})
	}
}

export { EventManager as FunctionManager }
