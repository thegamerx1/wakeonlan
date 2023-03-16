import { writable, get } from 'svelte/store';
import { devices, onlines } from './store';
import { dev } from '$app/environment';
export const status = writable({ connected: false, authenticated: false, error: false });
export const getStatus = () => get(status);

const API = dev ? 'ws://localhost:9000/ws' : 'wss://wake.ndrx.ml/ws';

let socketError = false;
let ws: WebSocket;
const resolves: any[number] = [];
let requestids = 1;
let waiting2Connect = false;
reset();
connect();

function reset() {
	status.set({ connected: false, authenticated: false, error: socketError });
}

function close() {
	ws.close();
}

function onError(_e: Event | CloseEvent) {
	if (waiting2Connect) return;
	if (ws.readyState === 0) return;
	reset();
	waiting2Connect = true;
	setTimeout(connect, socketError ? 2000 : 500);
}

function connect() {
	ws = new WebSocket(API);
	waiting2Connect = false;
	ws.onerror = (e) => {
		socketError = true;
		onError(e);
	};
	ws.onclose = onError;
	ws.onopen = () => {
		status.update((e) => {
			e.connected = true;
			return e;
		});
	};

	ws.onmessage = (event) => {
		let data: any;
		try {
			data = JSON.parse(event.data);
		} catch (e) {
			console.error('Server sent invalid data', data.data);
			return;
		}
		if (dev) console.log(data);
		switch (data.event) {
			case 'login':
				status.update((e) => {
					e.authenticated = data.success;
					return e;
				});

				resolves[data.nonce](data);
				delete resolves[data.nonce];
				break;

			case 'update':
				onlines.set(data.data);
				break;

			case 'wake':
				resolves[data.nonce](data);
				delete resolves[data.nonce];
				break;

			case 'save':
				resolves[data.nonce](data);
				delete resolves[data.nonce];
				break;

			case 'devices':
				devices.set(data.devices);
				break;

			default:
				console.error('Unkown event from server', data.event);
		}
	};
}

function wake(mac: string) {
	return new Promise((resolve) => {
		let nonce = sendEvent('wake', { mac });
		resolves[nonce] = resolve;
	});
}

function save(devices: Device[]) {
	return new Promise((resolve) => {
		devices = Array.isArray(devices) ? devices : [];
		let nonce = sendEvent('save', { devices });
		resolves[nonce] = resolve;
	});
}

function login(key: string | undefined) {
	return new Promise((resolve) => {
		let nonce = sendEvent('login', { login: key });
		resolves[nonce] = resolve;
	});
}

function sendEvent(event: string, obj: object) {
	let nonce = requestids++;
	ws.send(event + '|' + JSON.stringify({ ...obj, nonce }));
	return nonce;
}

export { wake, login, save, close };
