<script lang="ts">
	import { wake } from '../request';
	import { onMount } from 'svelte';
	import { slide, scale } from 'svelte/transition';
	import { createEventDispatcher } from 'svelte';
	import { remove, onlines } from '../store';
	import Wifioff from '$lib/icons/wifioff.svelte';
	import Wifi from '$lib/icons/wifi.svelte';
	import Spinner from '$lib/icons/spinner.svelte';
	const dispatch = createEventDispatcher();

	export let mac: string, name: string, host: string, index: number;

	const wakeTimeout = 2 * 60 * 1000;
	const confirmTimeout = 2.5 * 1000;
	const updateRate = 1000;

	let waitingon = false,
		errorWake = false,
		since = 0,
		online = 0,
		lastUpdate = 0,
		deletePromise: Promise<any>;
	let wakingPromise: Promise<any>,
		sinceTimer: number,
		deleteConfirm = false,
		wakeConfirm = false;

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
		if (!deleteConfirm) {
			deleteConfirm = true;
			setTimeout(() => {
				deleteConfirm = false;
			}, confirmTimeout);
			return;
		}
		deletePromise = remove(index);
	}

	function turnon() {
		if (!wakeConfirm) {
			wakeConfirm = true;
			setTimeout(() => {
				wakeConfirm = false;
			}, confirmTimeout);
			return;
		}
		wakingPromise = wake(mac).then((data: any) => {
			if (data.success) {
				waitingon = true;
				setTimeout(() => {
					waitingon = false;
				}, wakeTimeout);
			} else {
				errorWake = true;
				setTimeout(() => {
					errorWake = false;
				}, 5000);
			}
		});
	}
</script>

<div class="d-flex font-size-18 flex-wrap" in:scale={{ duration: 200 }}>
	<span class="px-5 font-weight-bold mr-auto">
		<div class="hover" on:click={() => dispatch('edit')}>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="24"
				height="24"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="feather feather-edit"
				><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" /><path
					d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
				/></svg
			>
		</div>
		{name}
	</span>
	<span class="text-muted font-size-16">
		{#if online === undefined}
			Loading <Spinner />
		{:else if waitingon && !online}
			Waiting to turn on <Spinner />
		{:else}
			{#if since > 15}
				<span>({since}s)</span>
			{/if}
			<span class={online ? 'text-success' : 'text-danger'}>
				{#if online}
					{online.toFixed(0)}ms <Wifi />
				{:else}
					Offline <Wifioff />
				{/if}
			</span>
		{/if}
	</span>
</div>
{#if online !== undefined}
	<div class="d-flex mt-15 buttonCont justify-content-end" in:slide>
		{#if !online && !waitingon}
			{#await wakingPromise}
				<button class="btn"><Spinner /></button>
			{:then}
				{#if errorWake}
					<button class="btn">Failed</button>
				{:else if wakeConfirm}
					<button class="btn btn-secondary" on:click={turnon}>Wake?</button>
				{:else}
					<button class="btn" on:click={turnon}>Wake</button>
				{/if}
				<span class="p-5" />
			{/await}
		{/if}
		{#await deletePromise}
			<button class="btn btn-danger"><Spinner /></button>
		{:then}
			{#if deleteConfirm}
				<button class="btn btn-danger" on:click={delet}>Delete?</button>
			{:else}
				<button class="btn" on:click={delet}>Delete</button>
			{/if}
		{/await}
	</div>
{/if}

<style lang="sass">
	.hover
		cursor: pointer
	.buttonCont button
		flex: 1

	:global(svg)
		display: inline

</style>
