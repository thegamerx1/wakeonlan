<script>
	import { ping, wake } from "../request"
	import { onMount } from "svelte"
	import { slide, scale } from "svelte/transition"
	import { createEventDispatcher } from "svelte"
	import { remove } from "../store"
	const dispatch = createEventDispatcher()

	export let mac, name, host, index

	const wakeTimeout = 2 * 60 * 1000
	const confirmTimeout = 2.5 * 1000
	const updateRate = 1000
	const aliveRate = 10 * 1000

	let waitingon, errorWake, since, online, lastUpdate, pingPromise, deletePromise
	let wakingPromise, sinceTimer, aliveTimer, deleteConfirm, wakeConfirm

	onMount(() => {
		sinceTimer = setInterval(sinceUpdate, updateRate)
		aliveTimer = setInterval(aliveUpdate, aliveRate)
		aliveUpdate()
		return () => {
			clearInterval(sinceTimer)
			clearInterval(aliveTimer)
		}
	})

	function aliveUpdate() {
		pingPromise = ping(host).then(data => {
			lastUpdate = performance.now()
			online = data.status
		})
	}

	function sinceUpdate() {
		since = Math.floor((performance.now() - lastUpdate) / 1000)
	}

	function delet() {
		if (!deleteConfirm) {
			deleteConfirm = true
			setTimeout(() => {
				deleteConfirm = false
			}, confirmTimeout)
			return
		}
		deletePromise = remove(index)
	}

	function turnon() {
		if (!wakeConfirm) {
			wakeConfirm = true
			setTimeout(() => {
				wakeConfirm = false
			}, confirmTimeout)
			return
		}
		wakingPromise = wake(mac).then(data => {
			if (data.success) {
				waitingon = true
				setTimeout(() => {
					waitingon = false
				}, wakeTimeout)
			} else {
				errorWake = true
				setTimeout(() => {
					errorWake = false
				}, 5000)
			}
		})
	}
</script>

<div class="d-flex font-size-18" in:scale={{ duration: 200 }}>
	<span class="px-5 font-weight-bold">
		<i class="fad fa-edit hover" on:click={() => dispatch("edit")} />
		{name}
	</span>
	<span class="text-muted ml-auto font-size-16">
		{#if online === undefined}
			Loading <i class="fad fa-spinner-third fa-spin" />
		{:else if waitingon && !online}
			Waiting to turn on <i class="fad fa-spinner-third fa-spin" />
		{:else}
			{#if since > 15}
				<span>({since}s)</span>
			{/if}
			<span class={online ? "text-success" : "text-danger"}>
				{#if online}
					{online.toFixed(0)}ms <i class="fad fa-signal" />
				{:else}
					Offline <i class="fad fa-signal-slash" />
				{/if}
			</span>
		{/if}
	</span>
</div>
{#if online !== undefined}
	<div class="d-flex mt-15 buttonCont justify-content-end" in:slide>
		{#if !online && !waitingon}
			{#await wakingPromise}
				<button class="btn"><i class="fas fa-circle-notch fa-spin" /></button>
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
			<button class="btn btn-danger"><i class="fas fa-circle-notch fa-spin" /></button>
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
</style>
