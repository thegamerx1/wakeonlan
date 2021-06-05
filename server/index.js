require("dotenv").config()
const WebSocket = require("ws")
const port = process.env.APP_PORT || 80
const wss = new WebSocket.Server({ port })
const funcs = require("./funcs")
wss.broadcast = obj => {
	wss.clients.forEach(client => {
		if (client.authenticated) client.Send(obj)
	})
}
funcs.init(wss)

wss.on("connection", ws => {
	ws.Send = obj => {
		if (ws.readyState === ws.OPEN) {
			ws.send(JSON.stringify(obj))
		}
	}
	ws.on("message", data => {
		try {
			data = JSON.parse(data)
		} catch (e) {
			console.error("Client sent invalid data", e)
			return
		}
		console.log(data)
		const event = data.event
		let found = false
		for (const key of Object.keys(funcs)) {
			if (event == key) {
				found = key
			}
		}
		if (found) {
			funcs[found](data, ws)
		} else {
			console.error("Unkown event from client", event)
		}
	})
})

wss.on("listening", () => console.log("Ready boi"))
wss.on("error", console.error)
