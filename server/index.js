require("dotenv").config()
const WebSocket = require("ws")
const port = process.env.port || 80
const wss = new WebSocket.Server({ port })
const funcs = require("./funcs")

wss.on("connection", ws => {
	ws.on("open", ws => {
		ws.send("shit on bitch")
	})

	ws.on("message", data => {
		try {
			data = JSON.parse(data)
		} catch {
			console.error("Client sent invalid data", data)
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
