import { writable, get } from 'svelte/store';
import { save } from './request';

const devices = writable<Device[]>([]);
interface Onlines {
	[key: string]: number;
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
		const current = get(devices).filter((e, i) => i !== index);
		save(current).then(resolve);
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
		if (validate(current, data)) return reject();
		current[index] = data;
		save(current).then(resolve);
	});
}

export { devices, onlines, remove, add, edit };
