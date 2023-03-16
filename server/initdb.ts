import JSONdb from "simple-json-db"

const CONFIG_DIR = "./data/"

export const db = new JSONdb(CONFIG_DIR + "devices.json")
import { existsSync, mkdirSync } from "fs"
if (!existsSync(CONFIG_DIR)) {
	mkdirSync(CONFIG_DIR)
}

if (!db.has("devices")) {
	db.set("devices", [])
}
