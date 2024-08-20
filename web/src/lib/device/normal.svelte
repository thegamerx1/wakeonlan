<script lang="ts">
	import { wake, shutdown } from '../request';
	import { onMount } from 'svelte';
	import { slide, scale } from 'svelte/transition';
	import { createEventDispatcher } from 'svelte';
	import { remove, onlines } from '../store';
	import halfmoon from 'halfmoon';
	import Wifioff from '$lib/icons/wifioff.svelte';
	import Wifi from '$lib/icons/wifi.svelte';
	import Spinner from '$lib/icons/spinner.svelte';
	import Confirm from '$lib/Confirm.svelte';
	const dispatch = createEventDispatcher();

	export let mac: string, name: string, host: string, api_key: string, index: number;

	const wakeTimeout = 2 * 60 * 1000;
	const updateRate = 1000;

	let waitingon: string | null,
		since = 0,
		online: number | null = 0,
		lastUpdate = 0,
		sinceTimer: number;

	onlines.subscribe((onlines) => {
		if (host in onlines) {
			online = onlines[host];
			lastUpdate = performance.now();
		}
	});

	onMount(() => {
		sinceTimer = setInterval(() => {
			since = Math.floor((performance.now() - lastUpdate) / 1000);
		}, updateRate);
		return () => {
			clearInterval(sinceTimer);
		};
	});

	function delet() {
		run('Deleting', remove(index));
	}

	function turnon() {
		run(
			'Waking',
			wake(mac).then((data: any) => {
				if (!data.success) {
					halfmoon.initStickyAlert({
						content: `Failed to wake ${name}`,
						title: 'Wake failed',
						alertType: 'alert-danger'
					});
				}
			})
		);
	}

	function run(message: string, promise: Promise<any>) {
		waitingon = message;
		let onfinish = () => {
			waitingon = null;
		};
		promise.then(onfinish).catch(onfinish);
	}

	function turnoff() {
		run(
			'Shutting down',
			shutdown(api_key).then((data: any) => {
				if (!data.success) {
					halfmoon.initStickyAlert({
						content: `Failed to shutdown ${name}`,
						title: 'Shutdown failed',
						alertType: 'alert-danger'
					});
				}
			})
		);
	}

	function edit() {
		dispatch('edit');
	}
</script>

<div class="font-size-18 flex flex-wrap" in:scale={{ duration: 200 }}>
	<div class="font-weight-bold mr-auto flex items-center px-5 uppercase">
		<svg
			on:click={edit}
			on:keyup={edit}
			tabindex="0"
			role="button"
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="hover mr-5"
			><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" /><path
				d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
			/></svg
		>
		{name}
	</div>
	<div class="text-muted font-size-16 flex items-center justify-center">
		{#if since > 15}
			<span>({since}s)</span>
		{/if}
		<span class={online == null ? 'text-danger' : 'text-success'}>
			{#if online == null}
				Offline <Wifioff />
			{:else}
				{online.toFixed(0)}ms <Wifi />
			{/if}
		</span>
	</div>
</div>

<div class="d-flex mt-15 buttonCont justify-content-end w-full" in:slide>
	{#if waitingon}
		<Spinner /> {waitingon}
	{:else}
		{#if online == null}
			<Confirm on:click={turnon} classes="btn-secondary" text="Wake" />
			<span class="p-5" />
		{:else}
			<Confirm on:click={turnoff} classes="btn-secondary" text="Shutdown" />
		{/if}
		<Confirm on:click={delet} classes="btn-secondary" text="Delete" />
	{/if}
</div>

<style lang="sass">
	.hover
		cursor: pointer
	.buttonCont :global(button)
		flex: 1
</style>
