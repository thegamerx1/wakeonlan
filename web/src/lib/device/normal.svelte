<script lang="ts">
	import { wake, shutdown } from '../request';
	import { onMount } from 'svelte';
	import { slide, scale } from 'svelte/transition';
	import { createEventDispatcher } from 'svelte';
	import { remove, onlines, type Online } from '../store';
	import halfmoon from 'halfmoon';
	import Wifioff from '$lib/icons/wifioff.svelte';
	import Wifi from '$lib/icons/wifi.svelte';
	import Spinner from '$lib/icons/spinner.svelte';
	import Confirm from '$lib/Confirm.svelte';
	import Edit from '$lib/icons/edit.svelte';
	import Trash from '$lib/icons/trash.svelte';
	const dispatch = createEventDispatcher();

	export let data: Device, index: number;

	const updateRate = 1000;

	let waitingon: string | null,
		since = 0,
		online: Online | null = null,
		lastUpdate = 0,
		sinceTimer: number;

	onlines.subscribe((onlines) => {
		if (data.host in onlines) {
			online = onlines[data.host];
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
			wake(data.mac).then((data: any) => {
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
			shutdown(data.host).then((data: any) => {
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

<div class="flex flex-col" in:scale={{ duration: 200 }}>
	<div class="flex justify-end">
		<div class="font-weight-bold mr-auto uppercase">
			{data.name}
		</div>

		<div
			class="flex items-center hover:text-gray-400"
			tabindex="0"
			role="button"
			on:click={edit}
			on:keyup={edit}
		>
			<Edit />
		</div>
		<div
			class="flex items-center hover:text-red-400"
			tabindex="0"
			role="button"
			on:click={delet}
			on:keyup={delet}
		>
			<Trash />
			<!-- <Confirm on:click={delet} classes="btn-secondary" text="Delete" /> -->
		</div>
	</div>
	<div class="text-muted flex w-full justify-end">
		{#if since > 15}
			<span>({since}s)</span>
		{/if}

		<span class="{online?.connected ? 'text-success' : 'text-gray-400'} px-5">
			{#if online?.connected}
				Agent connected
			{:else}
				Agent Disconnected
			{/if}
		</span>

		<span class={online?.latency == null ? 'text-danger' : 'text-success'}>
			{#if online?.latency == null}
				Offline <Wifioff />
			{:else}
				{online.latency.toFixed(0)}ms <Wifi />
			{/if}
		</span>
	</div>
</div>

<div class="d-flex mt-15 buttonCont justify-content-end w-full" in:slide>
	{#if waitingon}
		<Spinner /> {waitingon}
	{:else if online?.latency == null}
		<Confirm on:click={turnon} classes="btn-secondary" text="Wake" />
		<span class="p-5" />
	{:else if online?.connected}
		<Confirm
			on:click={turnoff}
			classes="btn-secondary {online?.connected ? 'btn-disabled' : ''}"
			text="Shutdown"
		/>
	{/if}
</div>

<style lang="sass">
	.hover
		cursor: pointer
	.buttonCont :global(button)
		flex: 1
</style>
