<script>
	import DevNormal from "./normal.svelte"
	import DevEditing from "./edit.svelte"
	import { scale } from "svelte/transition"

	let editing

	export let data, index
</script>

<div class="card m-10 w-300 px-25 d-flex flex-column align-items-top" transition:scale>
	{#if editing}
		<DevEditing {index} {data} on:cancel={() => (editing = false)} />
	{:else}
		<DevNormal {index} {...data} on:edit={() => (editing = true)} />
	{/if}
</div>
