<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

	export let confirmTimeout = 2.5 * 1000;
	export let text = 'confirm';
	export let classes = '';

	let confirm = false;

	function click() {
		if (!confirm) {
			confirm = true;
			setTimeout(() => {
				confirm = false;
			}, confirmTimeout);
			return;
		}
		dispatch('click');
	}
</script>

{#if confirm}
	<button class="btn {classes}" on:click={click}>{text}?</button>
{:else}
	<button class="btn" on:click={click}>{text}</button>
{/if}
