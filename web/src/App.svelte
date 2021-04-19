<script>
	import Device from "./Device.svelte"
	import AddModal from "./modal.svelte"
	import LoginPage from "./login.svelte"
	import { onMount } from "svelte"
	import { ping } from "./request.js"
	import { devices, getDevices } from "./store.js"
	import { save } from "./request"
	import { fly } from "svelte/transition"

	const refreshInterval = 10 * 1000
	let modalopen = false,
		onlines = [],
		authenticated = false

	onMount(() => {
		devices.subscribe(save2Cloud)
		setInterval(refresh, refreshInterval)
	})

	function save2Cloud() {
		if (authenticated) {
			refresh()
			save(getDevices())
		}
	}

	function removeDevice(index) {
		devices.update(arr => arr.splice(index, 1))
	}

	function logout() {
		localStorage.setItem("token", null)
		authenticated = false
	}

	function refresh() {
		ping(getDevices(devices).map(dev => dev.mac)).then(res => {
			if (res.success) {
				onlines = res.devices
				console.log(onlines)
			}
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
				<div class="my-10 px-10 justify-content-center d-flex justify-content-sm-end d-flex">
					<button class="btn" on:click={() => (modalopen = true)} in:fly>Add device</button>
				</div>
				<div class="d-flex p-15 flex-wrap flex-row justify-content-center">
					{#each $devices as dev, i (dev.mac)}
						<Device {...dev} bind:online={onlines[i]} on:delete={() => removeDevice(i)} />
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
