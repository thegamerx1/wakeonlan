import express from "express"
import { createServer } from "http"
import { z } from "zod"
import funcs from "./funcs"
import { WSServer, Lookup, WSSocket } from "./websocket"

const PORT = process.env.APP_PORT || 80

const app = express()
app.use(express.static(__dirname + "/public"))
const server = createServer(app)
app.use("*", (req, res) => {
	res.sendFile(__dirname + "/public/index.html")
})

const wss = new WSServer({ server, path: "/ws", WebSocket: WSSocket })

funcs.init(wss)

wss.on("connection", ws => {
	ws.heartbeat()
	ws.on("pong", ws.heartbeat.bind(ws))
	ws.on("message", data => {
		const [ev, json] = data.toString().split("|", 2)

		if (!(ev in Lookup)) {
			ws.invalid("event")
			return
		}
		const event = ev as keyof typeof Lookup

		let parse = Lookup[event]

		let params: z.infer<typeof parse>
		try {
			params = parse.parse(JSON.parse(json))
		} catch (err) {
			ws.invalid(err)
			return
		}

		if (!ws.authenticated) {
			if (event !== "login") {
				ws.invalid("Not logged in")
				return
			}
		}

		funcs[event](params as any, ws)
	})
})

const interval = setInterval(function ping() {
	wss.clients.forEach(function each(ws) {
		if (ws.isAlive === false) return ws.terminate()

		ws.isAlive = false
		ws.ping()
	})
}, 30000)

wss.on("close", function close() {
	clearInterval(interval)
})
server.listen(PORT, () => console.log(`Ready boi :${PORT}`))
wss.on("error", console.error)
