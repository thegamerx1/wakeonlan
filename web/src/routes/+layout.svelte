<script>
	import 'halfmoon/css/halfmoon-variables.min.css';

	import '../app.css';
	import halfmoon from 'halfmoon';
	import { onMount } from 'svelte';
	import PageTransition from '$lib/PageTransition.svelte';
	import { page } from '$app/stores';
	import AddModal from '$lib/Modal.svelte';
	import { status } from '$lib/request';

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
		{#if $status.authenticated}
			<button class="btn ml-auto" on:click={logout}>Logout</button>
		{/if}
	</nav>
	<div class="content-wrapper">
		<PageTransition url={$page.url.pathname}>
			<slot />
		</PageTransition>
	</div>
</div>
