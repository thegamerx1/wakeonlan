const api = PRODUCTION ? "https://lanapi.ndrx.ml/" : "http://localhost:9000/"

async function make(point, data) {
	const res = await fetch(api + point, {
		method: "post",
		headers: {
			"Content-Type": "application/json",
			key: localStorage.getItem("token"),
		},
		body: JSON.stringify(data),
		mode: "cors",
	})
	if (res.ok) {
		return res.json()
	} else {
		throw null
	}
}

async function ping(hosts) {
	const res = await make("ping", { hosts })
	return res
}

async function wake(mac) {
	const res = await make("wake", { mac })
	return res
}

async function save(devices) {
	devices = Array.isArray(devices) ? devices : []
	const res = await make("save", { devices })
	return res
}

async function login(key) {
	if (typeof key === "undefined") key = localStorage.getItem("token")
	const res = await make("login", { key })
	if (res.success) localStorage.setItem("token", key)
	return res
}

export { ping, wake, login, save }
