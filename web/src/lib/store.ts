import { writable, get } from 'svelte/store';
import { save } from './request';

const devices = writable<Device[]>([]);
export type Online = {
	latency: number | null;
	connected: boolean;
};

export interface Onlines {
	[key: string]: Online;
}

const onlines = writable<Onlines>({});
function validate(current: Device[], data: Device) {
	for (let i = 0; i < current.length; i++) {
		if (current[i].host === data.host) return true;
	}
	return false;
}

function remove(index: number) {
	return new Promise((resolve) => {
		save(get(devices).filter((e, i) => i !== index)).then(resolve);
	});
}

function add(data: Device) {
	return new Promise((resolve, reject) => {
		const current = get(devices);
		if (validate(current, data)) return reject();
		save([...current, data]).then(resolve);
	});
}

function edit(index: number, data: Device) {
	return new Promise((resolve, reject) => {
		const current = get(devices);
		current.splice(index, 1);
		if (validate(current, data)) {
			current.unshift(data);
			return reject();
		}
		current.unshift(data);
		save(current).then(resolve);
	});
}

export { devices, onlines, remove, add, edit };
