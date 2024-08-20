import { writable, get } from 'svelte/store';
import { devices, onlines } from './store';
import { dev } from '$app/environment';

export const status = writable({ connected: false, authenticated: false, error: false });
export const getStatus = () => get(status);

export const passwordStore = writable('');
const API = `${window.location.protocol == 'https:' ? 'wss' : 'ws'}://${window.location.host}/ws`;

let socketError = false;
let ws: WebSocket;
const resolves: any[number] = [];
let requestids = 1;
reset();
connect();
let password = '';
passwordStore.subscribe((pass) => () => (password = pass));

function reset() {
	if (ws) ws.close();
	status.set({ connected: false, authenticated: false, error: socketError });
}

function onError(_e: Event | CloseEvent) {
	if (ws.readyState === 0) return;
	reset();
	setTimeout(connect, socketError ? 2000 : 500);
}

function connect() {
	ws = new WebSocket(API);
	ws.onerror = (e) => {
		socketError = true;
		onError(e);
	};
	ws.addEventListener('ping', () => console.log('gilipollas'));
	ws.onclose = onError;
	ws.onopen = () => {
		let auth = false;
		if (password) {
			login(password)
				.then(() => {
					auth = true;
				})
				.catch(() => {
					auth = false;
				});
		}
		status.update((e) => {
			e.connected = true;
			e.authenticated = auth;
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
				devices.set(data.data);
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

function login(login: string) {
	return new Promise((resolve) => {
		let nonce = sendEvent('login', { login });
		resolves[nonce] = resolve;
	}).then((data: any) => {
		devices.set(Array.isArray(data.devices) ? data.devices : []);
		return data;
	});
}

function sendEvent(event: string, obj: object) {
	let nonce = requestids++;
	ws.send(JSON.stringify({ ...obj, event: event, nonce }));
	return nonce;
}

export { wake, login, save };
