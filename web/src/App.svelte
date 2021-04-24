<script>
	import { fly, fade } from "svelte/transition"
	import DevicesPage from "./device/page.svelte"
	import AddModal from "./modal.svelte"
	import LoginPage from "./login.svelte"
	import { status, close } from "./request"

	let modalopen = false

	function logout() {
		localStorage.setItem("token", null)
		close()
	}
</script>

<AddModal bind:show={modalopen} />
<div class="page-wrapper with-navbar">
	<div class="sticky-alerts" />
	<nav class="navbar">
		{#if $status.authenticated}
			<button class="btn ml-auto" on:click={logout}>Logout</button>
		{/if}
	</nav>

	<div class="content-wrapper d-flex justify-content-center align-items-center" transition:fly>
		{#if $status.connected}
			{#if $status.authenticated}
				<DevicesPage on:modal={() => (modalopen = true)} />
			{:else}
				<LoginPage />
			{/if}
		{:else if $status.error}
			<div class="alert my-5 font-size-16 alert-danger" transition:fade={{ duration: 500 }}>
				<i class="fad fa-spinner-third fa-spin" />
				Connection to server failed..
			</div>
		{:else}
			<i class="fad fa-spinner-third fa-spin fa-3x" />
		{/if}
	</div>
</div>
