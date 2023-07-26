<script>
	import { fly } from 'svelte/transition';
	import { status } from '$lib/request';
	import { devices } from '$lib/store';
	import { goto } from '$app/navigation';
	import Device from '$lib/device/device.svelte';
	import { showModal } from '$lib/Modal.svelte';
	import Spinner from '$lib/icons/spinner.svelte';

	$: {
		if (!$status.authenticated) {
			goto('/login');
		}
	}
</script>

<div class="my-10 p-10">
	<button class="btn" on:click={() => showModal.set(true)} in:fly>Add device</button>
</div>
<div class="flex p-15 flex-wrap flex-row justify-content-center align-items-start">
	{#if $status.connected}
		{#each $devices as data, i (data.host)}
			<Device {data} index={i} />
		{:else}
			No devices!
		{/each}
	{:else}
		<Spinner />
	{/if}
</div>
