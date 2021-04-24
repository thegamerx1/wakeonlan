import { writable, get } from "svelte/store"
import { save } from "./request"

const toCheck = ["name", "mac", "host"]

const devices = writable([])

devices.subscribe(console.log)

function validate(current, data) {
	for (var i = 0; i < current.length; i++) {
		if (current[i]["host"] === data["host"]) return true
	}
	return false
}

function remove(index) {
	return new Promise(resolve => {
		const current = get(devices).filter((e, i) => i !== index)
		save(current).then(data => {
			devices.set(data.devices)
			resolve()
		})
	})
}

function add(data) {
	return new Promise((resolve, reject) => {
		const current = get(devices)
		if (validate(current, data)) return reject()
		const promise = save([...current, data])
		promise.then(data => {
			devices.set(data.devices)
			resolve()
		})
	})
}

function edit(index, data) {
	return new Promise((resolve, reject) => {
		const current = get(devices)
		current.splice(index, 1)
		if (validate(current, data)) return reject()
		current[index] = data
		console.log(current, data)
		save(current).then(data => {
			devices.set(data.devices)
			resolve()
		})
	})
}

export { devices, remove, add, edit, toCheck }
