<script lang="ts" context="module">
	import { writable } from 'svelte/store';

	export let showModal = writable(false);
</script>

<script lang="ts">
	import Spinner from './icons/spinner.svelte';
	import { add } from './store';

	const defaultDevice = { host: '', mac: '', name: '', api_key: null };
	let data = { ...defaultDevice },
		error = false,
		submiting = false,
		input: HTMLElement;

	$: if ($showModal) {
		reset();
		input.focus();
	}
	reset();

	function onSubmit() {
		submiting = true;
		add(data)
			.then((data: any) => {
				if (data.success) {
					$showModal = false;
				} else {
					error = true;
					submiting = false;
				}
			})
			.catch((e) => {
				error = true;
				submiting = false;
			});
	}

	function reset() {
		error = submiting = false;
		data = { ...defaultDevice };
	}
</script>

<div
	class="modal"
	class:show={$showModal}
	hidden={!$showModal}
	role="dialog"
	data-overlay-dismissal-disabled="true"
	data-esc-dismissal-disabled="true"
>
	<div class="modal-dialog" role="document">
		<div class="modal-content">
			<h5 class="modal-title">Add device</h5>
			<form on:submit|preventDefault={onSubmit}>
				<label class="w-full">
					Name
					<input
						type="text"
						bind:this={input}
						bind:value={data.name}
						class="form-control"
						placeholder="Name"
						disabled={submiting}
						maxlength="18"
					/>
				</label>
				<label class="w-full"
					>MAC Address
					<input
						type="text"
						bind:value={data.mac}
						class="form-control"
						placeholder="00:00:00:00:00"
						disabled={submiting}
						maxlength="17"
					/>
				</label>
				<label class="w-full"
					>Host
					<input
						type="text"
						bind:value={data.host}
						class="form-control"
						placeholder="192.168.1.1"
						disabled={submiting}
						maxlength="18"
					/>
				</label>
				{#if error}
					<div class="invalid-feedback">Error</div>
				{/if}
				<div class="buttons mt-20">
					<button
						class="btn mr-5"
						type="button"
						disabled={submiting}
						on:click={() => showModal.set(false)}>Close</button
					>
					<button
						class="btn btn-primary w-100 flex items-center justify-center"
						type="submit"
						disabled={submiting}
					>
						{#if submiting}
							<Spinner />
						{:else}
							Add
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
</div>

<style lang="sass">
	.buttons
		@apply flex flex-row xl:justify-end
		button
			@apply flex-1 xl:flex-initial
</style>
