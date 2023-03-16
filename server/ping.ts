import icmp from "ping"
import { performance } from "perf_hooks"

export function ping(host: string): Promise<number> {
	return new Promise(resolve => {
		let resolved = false
		const start = performance.now()
		const send = (out: number) => {
			if (resolved) return
			resolved = true
			resolve(out)
		}
		icmp.sys.probe(host, alive => {
			send(alive ? performance.now() - start : 0)
		})
		setTimeout(() => {
			send(0)
		}, 500)
	})
}
