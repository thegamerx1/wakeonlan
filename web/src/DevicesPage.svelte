<script>
	import { devices, getDevices } from "./store"
	import { save } from "./request"
	import { onMount, createEventDispatcher } from "svelte"
	import { fly } from "svelte/transition"
	import Device from "./Device.svelte"
	const dispatch = createEventDispatcher()

	function remove(index) {
		console.log("remove", index)
		const result = $devices.filter((e, i) => i !== index)
		save(result).then(data => {
			devices.set(data.devices)
		})
	}
</script>

<div class="container-fluid h-full">
	<div
		class="card my-10 p-10 justify-content-center d-flex align-items-center justify-content-sm-end d-flex"
	>
		<button class="btn" on:click={() => dispatch("modal")} in:fly>Add device</button>
	</div>
	<div class="d-flex p-15 flex-wrap flex-row justify-content-center">
		{#each $devices as dev, i (dev.mac)}
			<Device {...dev} on:remove={() => remove(i)} />
		{:else}
			No devices!
		{/each}
	</div>
</div>
