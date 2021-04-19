import { writable, get } from 'svelte/store'
export const devices = writable([])
export const getDevices = () => get(devices)