const wol = require("wakeonlan")
const JSONdb = require("simple-json-db")
const configDir = "./data/"
const db = new JSONdb(configDir + "devices.json")
const fs = require("fs")
const { ping } = require("./ping")

if (!fs.existsSync(configDir)) {
	fs.mkdirSync(configDir)
}

const needed = ["name", "mac", "host"]

function wake(req, res) {
	wol(req.body.mac)
		.then(() => {
			res.json({ success: true })
		})
		.catch(e => {
			res.json({ success: false, error: e })
		})
}

function aliveCheck(req, res) {
	const promises = []
	req.body.hosts.forEach(host => {
		promises.push(ping(host))
	})
	Promise.all(promises).then(out => {
		res.json({ devices: out })
	})
}

function save(req, res) {
	const data = Array.isArray(req.body.devices) ? req.body.devices : []
	data.forEach((e, i) => {
		needed.forEach(check => {
			if (typeof e[check] !== "string" || e[check] === "") {
				data.splice(i, 1)
			}
		})
	})
	db.set("devices", data)
	db.sync()
	res.json({ devices: data })
}

function login(req, res) {
	const success = req.body.key === process.env.key
	res.json({ success: success, devices: success ? db.get("devices") : null })
}

module.exports = { aliveCheck, wake, save, login }
