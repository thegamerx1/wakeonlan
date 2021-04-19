const wol = require("wakeonlan")
const JSONdb = require("simple-json-db")
const db = new JSONdb("./config.json")
const { exec } = require("child_process")
var devices = []

refresh()
function refresh() {
	exec("arp -a", (error, stdout) => {
		if (error) return console.error(error)
		devices = stdout.match(/(?<mac>(\w{2}[:-]){5}\w{2})/g)
		devices.forEach((e, i) => {
			devices[i] = e.replace(/-/g, ":")
		})
		console.log(devices)
	})
}

function wake(req, res) {
	wol(req.body.mac)
		.then(() => {
			res.json({ success: true })
		})
		.catch(e => {
			res.json({ success: false, error: e })
		})
}

function ping(req, res) {
	const out = []
	req.body.hosts.forEach((host, index) => {
		out[index] = false
		for (const value of devices) {
			if (value == host) {
				out[index] = true
			}
		}
	})
	res.json({ devices: out })
}

function save(req, res) {
	const data = Array.isArray(req.body.devices) ? req.body.devices : []
	db.set("devices", data)
	db.sync()
}

function login(req, res) {
	const success = req.body.key === process.env.key
	res.json({ success: success, devices: success ? db.get("devices") : null })
}

module.exports = { ping, wake, save, login }
