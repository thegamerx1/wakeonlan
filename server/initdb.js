const JSONdb = require("simple-json-db")
const configDir = "./data/"
const db = new JSONdb(configDir + "devices.json")
const fs = require("fs")
if (!fs.existsSync(configDir)) {
	fs.mkdirSync(configDir)
}
module.exports = db
