import { writable, get } from "svelte/store"
import { devices, onlines } from "./store"
export const status = writable([])
export const getStatus = () => get(status)

const API = PRODUCTION ? "wss://lanapi.ndrx.ml" : "ws://localhost:9000"

let socketError = false
var ws
const resolves = []
let requestids = 1
let waiting2Connect = false
reset()
connect()

function reset() {
	status.set({ connected: false, authenticated: false, error: socketError })
}

function close() {
	ws.close()
}

function onError(e) {
	if (waiting2Connect) return
	if (ws.readyState === 0) return
	reset()
	waiting2Connect = true
	setTimeout(connect, socketError ? 2000 : 500)
}

function connect() {
	ws = new WebSocket(API)
	waiting2Connect = false
	ws.onerror = e => {
		socketError = true
		onError(e)
	}
	ws.onclose = onError
	ws.onopen = () => {
		status.update(e => {
			e.connected = true
			return e
		})
	}

	ws.onmessage = data => {
		try {
			data = JSON.parse(data.data)
		} catch (e) {
			console.error("Server sent invalid data", data.data)
			return
		}
		if (!PRODUCTION) console.log(data)
		switch (data.event) {
			case "login":
				status.update(e => {
					e.authenticated = data.success
					return e
				})

				resolves[data.nonce](data)
				delete resolves[data.nonce]
				onlines.set(data.onlines)
				break

			case "update":
				onlines.set(data.data)
				break

			case "wake":
				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			case "save":
				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			case "devices":
				devices.set(data.devices)
				break

			default:
				console.error("Unkown event from server", data.event)
		}
	}
}

function wake(mac) {
	return new Promise(resolve => {
		let nonce = requestids++
		ws.send(JSON.stringify({ event: "wake", mac, nonce }))
		resolves[nonce] = resolve
	})
}

function save(devices) {
	return new Promise(resolve => {
		let nonce = requestids++
		devices = Array.isArray(devices) ? devices : []
		ws.send(JSON.stringify({ event: "save", devices, nonce }))
		resolves[nonce] = resolve
	})
}

function login(key) {
	return new Promise(resolve => {
		let nonce = requestids++
		if (typeof key === "undefined") key = localStorage.getItem("token")
		ws.send(JSON.stringify({ event: "login", login: key, nonce }))
		resolves[nonce] = resolve
	})
}

export { wake, login, save, close }
