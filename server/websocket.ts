import { WebSocketServer, WebSocket } from "ws"
import { z } from "zod"

export class WSSocket extends WebSocket {
	authenticated = false
	isAlive = false

	Send(obj: object) {
		if (this.readyState === this.OPEN) {
			this.send(JSON.stringify(obj))
		}
	}

	heartbeat() {
		this.isAlive = true
	}

	invalid(err: any) {
		console.error("Client sent invalid data", err)
		this.terminate()
	}
}

export class WSServer extends WebSocketServer<WSSocket> {
	broadcast(obj: object) {
		this.clients.forEach(client => {
			if (client.authenticated) {
				client.Send(obj)
			}
		})
	}
}

const deviceSchema = z.object({
	name: z.string().max(18).min(1),
	mac: z.string(),
	host: z.string(),
})

const saveEventSchema = z.object({
	devices: z.array(deviceSchema),
	nonce: z.number(),
})
const wakeEventSchema = z.object({
	mac: z.string(),
	nonce: z.number(),
})

const loginEventSchema = z.object({
	login: z.string().nonempty(),
	nonce: z.number(),
})

export const Lookup = {
	login: loginEventSchema,
	wake: wakeEventSchema,
	save: saveEventSchema,
}

export const eventSchema = z.union([wakeEventSchema, saveEventSchema, loginEventSchema])
export type Device = z.infer<typeof deviceSchema>
export type WakeEvent = z.infer<typeof wakeEventSchema>
export type SaveEvent = z.infer<typeof saveEventSchema>
export type LoginEvent = z.infer<typeof loginEventSchema>

export type Event = z.infer<typeof eventSchema>
