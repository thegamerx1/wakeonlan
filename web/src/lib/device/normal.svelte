<script lang="ts">
	import { wake } from '../request';
	import { onMount } from 'svelte';
	import { slide, scale } from 'svelte/transition';
	import { createEventDispatcher } from 'svelte';
	import { remove, onlines } from '../store';
	import Wifioff from '$lib/icons/wifioff.svelte';
	import Wifi from '$lib/icons/wifi.svelte';
	import Spinner from '$lib/icons/spinner.svelte';
	import Confirm from '$lib/Confirm.svelte';
	const dispatch = createEventDispatcher();

	export let mac: string, name: string, host: string, index: number;

	const wakeTimeout = 2 * 60 * 1000;
	const updateRate = 1000;

	let waitingon = false,
		errorWake = false,
		since = 0,
		online: number | null = 0,
		lastUpdate = 0,
		deletePromise: Promise<any>;
	let wakingPromise: Promise<any>, sinceTimer: number;

	onlines.subscribe((onlines) => {
		if (host in onlines) {
			online = onlines[host] ?? null;
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
		deletePromise = remove(index);
	}

	function turnon() {
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

	function edit() {
		dispatch('edit');
	}
</script>

<div class="flex font-size-18 flex-wrap" in:scale={{ duration: 200 }}>
	<div class="flex items-center px-5 font-weight-bold mr-auto uppercase">
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
		{#if online === undefined}
			<Spinner /> Loading
		{:else if waitingon && online == null}
			<Spinner /> Waiting to turn on
		{:else}
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
		{/if}
	</div>
</div>
{#if online !== undefined}
	<div class="d-flex mt-15 buttonCont justify-content-end" in:slide>
		{#if online == null && !waitingon}
			{#await wakingPromise}
				<button class="btn flex items-center justify-center"><Spinner /></button>
			{:then}
				{#if errorWake}
					<button class="btn">Failed</button>
				{:else}
					<Confirm on:click={turnon} classes="btn-secondary" text="Wake" />
				{/if}
			{/await}
			<span class="p-5" />
		{/if}
		{#await deletePromise}
			<button class="btn btn-danger flex items-center justify-center"><Spinner /></button>
		{:then}
			<Confirm on:click={delet} classes="btn-secondary" text="Delete" />
		{/await}
	</div>
{/if}

<style lang="sass">
	.hover
		cursor: pointer
	.buttonCont :global(button)
		flex: 1
</style>
