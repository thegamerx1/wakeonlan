import { writable, get } from "svelte/store"
export const status = writable([])
export const getStatus = () => get(status)

const API = PRODUCTION ? "wss://lanapi.ndrx.ml/ws/" : "ws://localhost:9000"

let socketError = false
var ws
const resolves = []
let requestids = 1
reset()
connect()

function reset() {
	status.set({ connected: false, authenticated: false, error: socketError })
}

function close() {
	ws.close()
}

function onError(e) {
	console.error(e)
	reset()
	setTimeout(connect, socketError ? 2000 : 500)
}

function connect() {
	ws = new WebSocket(API)
	console.log("Openning connection")
	ws.onerror = e => {
		socketError = true
		onError(e)
	}
	ws.onclose = onError
	ws.onopen = () => {
		console.log("Connected")
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
		console.log(data)
		switch (data.event) {
			case "login":
				status.update(e => {
					e.authenticated = data.success
					return e
				})

				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			case "ping":
				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			case "wake":
				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			case "save":
				resolves[data.nonce](data)
				delete resolves[data.nonce]
				break

			default:
				console.error("Unkown event from server", data.event)
		}
	}
}

async function make(point, data) {
	const res = await fetch(API + point, {
		method: "post",
		headers: {
			"Content-Type": "application/json",
			key: localStorage.getItem("token"),
		},
		body: JSON.stringify(data),
		mode: "cors",
	})
	if (res.ok) {
		return res.json()
	} else {
		throw null
	}
}

function ping(host) {
	return new Promise(resolve => {
		let nonce = requestids++
		ws.send(JSON.stringify({ event: "ping", host, nonce }))
		resolves[nonce] = resolve
	})
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

export { ping, wake, login, save, close }
