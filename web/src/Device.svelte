<script>
	import { ping, wake } from "./request"
	import { onMount } from "svelte"
	import { scale, slide } from "svelte/transition"
	import { createEventDispatcher } from "svelte"
	const dispatch = createEventDispatcher()

	export let mac, name, host
	const wakeTimeout = 2 * 60 * 1000
	const confirmTimeout = 2.5 * 1000
	const updateRate = 1000
	const aliveRate = 10 * 1000

	let waitingon, errorWake, since, online, lastUpdate, pingPromise
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

	function remove() {
		if (!deleteConfirm) {
			deleteConfirm = true
			setTimeout(() => {
				deleteConfirm = false
			}, confirmTimeout)
			return
		}
		dispatch("remove")
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

<div class="card m-10 w-300 px-25 d-flex flex-column" transition:scale>
	<div class="d-flex font-size-18">
		<span class="px-5 font-weight-bold">{name}</span>
		<span class="text-muted ml-auto font-size-16">
			{#if online === undefined}
				Loading <i class="fad fa-spinner-third fa-spin" />
			{:else if waitingon}
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
			{#if deleteConfirm}
				<button class="btn btn-danger" on:click={remove}>Delete?</button>
			{:else}
				<button class="btn" on:click={remove}>Delete</button>
			{/if}
		</div>
	{/if}
</div>

<style lang="sass">
	.buttonCont button
		flex: 1
</style>
