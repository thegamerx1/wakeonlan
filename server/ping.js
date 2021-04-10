const icmp = require("ping")
const { performance } = require("perf_hooks")

function ping(host) {
	return new Promise(resolve => {
		let resolved = false
		const start = performance.now()
		const send = out => {
			if (resolved) return
			resolved = true
			resolve(out)
		}
		icmp.sys.probe(host, alive => {
			send(alive ? performance.now() - start : alive)
		})
		setTimeout(() => {
			send(false)
		}, 500)
	})
}

module.exports = { ping }
