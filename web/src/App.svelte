<script>
	import { fly } from "svelte/transition"
	import { devices, getDevices } from "./store.js"
	import { onMount } from "svelte"
	import Device from "./Device.svelte"
	import AddModal from "./modal.svelte"
	import LoginPage from "./login.svelte"
	import { ping, save } from "./request"

	const refreshInterval = 10 * 1000
	let modalopen = false,
		onlines = [],
		authenticated = false,
		lastUpdate,
		isSaving,
		refreshing

	onMount(() => {
		devices.subscribe(save2Cloud)
		setInterval(refresh, refreshInterval)
	})

	function save2Cloud() {
		if (isSaving) return
		if (authenticated) {
			refresh()
			isSaving = true
			save(getDevices()).then(res => {
				devices.set(res.devices)
				isSaving = false
			})
		}
	}

	function remove(index) {
		console.log("remove", index)
		devices.update(arr => arr.filter((e, i) => i !== index))
	}

	function logout() {
		localStorage.setItem("token", null)
		authenticated = false
	}

	function refresh() {
		const devs = getDevices(devices).map(dev => dev.host)
		if (devs.length === 0) return

		refreshing = ping(devs).then(res => {
			onlines = res.devices
			lastUpdate = performance.now()
		})
	}
</script>

<AddModal bind:show={modalopen} />
<div class="page-wrapper with-navbar">
	<div class="sticky-alerts" />
	<nav class="navbar">
		{#if authenticated}
			<button class="btn ml-auto" on:click={logout}>Logout</button>
		{/if}
	</nav>

	<div class="content-wrapper">
		{#if authenticated}
			<div>
				<div
					class="card my-10 p-10 justify-content-center d-flex align-items-center justify-content-sm-end d-flex"
				>
					{#await refreshing}
						<span class="px-5">Refreshing <i class="fad fa-spinner-third fa-spin" /></span>
					{:catch}
						<span class="text-danger">Error refreshing</span>
					{/await}
					<button class="btn" on:click={() => (modalopen = true)} in:fly>Add device</button>
				</div>
				<div class="d-flex p-15 flex-wrap flex-row justify-content-center">
					{#each $devices as dev, i (dev.mac)}
						<Device {...dev} online={onlines[i]} on:remove={() => remove(i)} {lastUpdate} />
					{:else}
						No devices!
					{/each}
				</div>
			</div>
		{:else}
			<div class="position-fixed right-0 left-0 top-0 bottom-0 bg-dark-light" transition:fly>
				<LoginPage bind:logged={authenticated} />
			</div>
		{/if}
	</div>
</div>
