<script>
	import { devices } from "../store"
	import { createEventDispatcher } from "svelte"
	import { fly } from "svelte/transition"
	import Device from "./device.svelte"
	const dispatch = createEventDispatcher()
</script>

<div class="container-fluid h-full">
	<div
		class="card my-10 p-10 justify-content-center d-flex align-items-center justify-content-sm-end d-flex"
	>
		<button class="btn" on:click={() => dispatch("modal")} in:fly>Add device</button>
	</div>
	<div class="d-flex p-15 flex-wrap flex-row justify-content-center align-items-start">
		{#each $devices as data, i (data.host)}
			<Device {data} index={i} />
		{:else}
			No devices!
		{/each}
	</div>
</div>
