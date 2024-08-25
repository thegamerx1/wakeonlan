<script lang="ts">
	import DevNormal from './normal.svelte';
	import DevEditing from './edit.svelte';
	import { scale } from 'svelte/transition';

	let editing = false;

	export let data: Device, index: number;
</script>

<div class="font-size-16 card w-300 d-flex flex-column m-10" transition:scale>
	{#if editing}
		<DevEditing {index} {data} on:cancel={() => (editing = false)} />
	{:else}
		<DevNormal {index} {data} on:edit={() => (editing = true)} />
	{/if}
</div>
