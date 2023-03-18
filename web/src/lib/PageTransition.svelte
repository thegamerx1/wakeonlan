<script>
	import { scale } from 'svelte/transition';
	export let url = '';
	const pageTransitionDuration = 500;
</script>

{#key url}
	<div
		class="h-full"
		in:scale={{ duration: pageTransitionDuration, delay: pageTransitionDuration }}
		out:scale={{ duration: pageTransitionDuration }}
	>
		<slot />
	</div>
{/key}

<style>
	div {
		overflow: hidden;
	}
</style>
