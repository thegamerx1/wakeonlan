<script>
	import 'halfmoon/css/halfmoon-variables.min.css';

	import '../app.css';
	import halfmoon from 'halfmoon';
	import { onMount } from 'svelte';
	import PageTransition from '$lib/PageTransition.svelte';
	import { page } from '$app/stores';
	import AddModal from '$lib/Modal.svelte';
	import { status } from '$lib/request';
	import { fly } from 'svelte/transition';
	import { showModal } from '$lib/Modal.svelte';

	onMount(() => {
		halfmoon.onDOMContentLoaded();
	});

	function logout() {
		localStorage.setItem('token', '');
		window.location.reload();
	}
</script>

<svelte:head>
	<title>Wake</title>
</svelte:head>

<AddModal />

<div class="page-wrapper with-navbar">
	<div class="sticky-alerts" />
	<nav class="navbar">
		{#if $status.loaded}
			<button class="btn" on:click={() => showModal.set(true)} in:fly>Add device</button>
			<button class="btn ml-auto" on:click={logout} in:fly>Logout</button>
		{/if}
	</nav>
	<div class="content-wrapper">
		<PageTransition url={$page.url.pathname}>
			<slot />
		</PageTransition>
	</div>
</div>
