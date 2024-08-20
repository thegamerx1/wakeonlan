<script lang="ts">
	import { edit } from '../store';
	import { createEventDispatcher } from 'svelte';
	import Spinner from '$lib/icons/spinner.svelte';
	const dispatch = createEventDispatcher();

	export let data: Device, index: number;

	let error = false,
		submiting = false,
		promise: Promise<void>,
		newdata = { ...data };

	function clipboard() {
		navigator.clipboard.writeText(newdata.api_key ?? '');
	}

	function regenerate() {
		newdata.api_key = crypto.randomUUID() + crypto.randomUUID();
	}

	function focus(e: HTMLElement) {
		e.focus();
	}

	function change(
		what: keyof Device,
		e: Event & {
			currentTarget: EventTarget & HTMLInputElement;
		}
	) {
		let t = e.currentTarget;
		if (t.value === '' || t.value === ' ') {
			t.value = data[what] ?? '';
		}
	}

	function onSubmit() {
		error = false;
		submiting = true;
		promise = edit(index, newdata)
			.then(() => {
				dispatch('cancel');
			})
			.catch((e) => {
				console.error(e);
				error = true;
			});
	}
</script>

<div class="font-size-18">
	{#if error}
		<div class="alert alert-danger" role="alert">Duplicate data found</div>
	{/if}
	<form on:submit|preventDefault={onSubmit} class="font-size-16">
		<label class="w-full">
			Name
			<input
				type="text"
				use:focus
				bind:value={newdata.name}
				class="form-control"
				on:change={(e) => change('name', e)}
				disabled={submiting}
				maxlength="18"
			/>
		</label>
		<label class="w-full">
			MAC Address
			<input
				type="text"
				bind:value={newdata.mac}
				class="form-control"
				on:change={(e) => change('mac', e)}
				disabled={submiting}
				maxlength="17"
			/>
		</label>
		<label class="w-full">
			Host
			<input
				type="text"
				bind:value={newdata.host}
				class="form-control"
				on:change={(e) => change('host', e)}
				disabled={submiting}
				maxlength="18"
			/>
		</label>
		<label class="w-full">
			API KEY
			{#if newdata.api_key}
				<input
					type="text"
					value={newdata.api_key.substring(0, 5) + '-*****'.repeat(3)}
					class="form-control"
					disabled={true}
					maxlength="5"
				/>
			{/if}
			<div class="flex w-full flex-row">
				<button class="btn flex-1" type="button" disabled={submiting} on:click={() => regenerate()}
					>Regenerate</button
				>
				{#if newdata.api_key}
					<button class="btn flex-1" type="button" disabled={submiting} on:click={() => clipboard()}
						>Copy</button
					>
				{/if}
			</div>
		</label>
		<div class="mt-20 flex text-right">
			<button
				class="btn mr-5 flex-1"
				type="button"
				disabled={submiting}
				on:click={() => dispatch('cancel')}>Cancel</button
			>
			<button
				class="btn btn-primary flex flex-1 items-center justify-center"
				type="submit"
				disabled={submiting}
			>
				{#await promise}
					<Spinner />
				{:then}
					Done
				{/await}
			</button>
		</div>
	</form>
</div>
