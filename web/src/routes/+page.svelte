<script>
	import { status } from '$lib/request';
	import { devices } from '$lib/store';
	import { goto } from '$app/navigation';
	import Device from '$lib/device/device.svelte';
	import Spinner from '$lib/icons/spinner.svelte';

	$: {
		if (!$status.authenticated) {
			goto('/login');
		}
	}
</script>

<div class="p-15 justify-content-center align-items-start flex flex-row flex-wrap">
	{#if $status.loaded}
		{#each $devices as data, i (data.host)}
			<Device {data} index={i} />
		{:else}
			No devices!
		{/each}
	{:else}
		<Spinner />
	{/if}
</div>
