const wol = require("wakeonlan")
const JSONdb = require("simple-json-db")
const find = require("local-devices")
const db = new JSONdb("./config.json")
var devices = []

refresh()
function refresh() {
	find().then(devs => {
		devices = devs
		setTimeout(refresh, 5 * 1000)
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
			console.log(value.mac + "  " + host)
			if (value.mac == host) {
				out[index] = true
			}
		}
	})
	console.log(out)
	res.json({ success: true, devices: out })
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
