<script>
	import { edit } from "../store"
	import { createEventDispatcher } from "svelte"
	const dispatch = createEventDispatcher()

	let error, submiting, promise
	export let data, index
	let newdata = { ...data }

	function focus(e) {
		e.focus()
	}

	function change(what, e) {
		e = e.target
		if (e.value === "" || e.value === " ") e.value = data[what]
	}

	function onSubmit() {
		error = false
		submiting = true
		promise = edit(index, newdata)
			.then(() => {
				dispatch("cancel")
			})
			.catch(e => {
				console.error(e)
				error = true
			})
	}
</script>

<div class="font-size-18" z>
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
				on:change={e => change("name", e)}
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
				on:change={e => change("mac", e)}
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
				on:change={e => change("host", e)}
				disabled={submiting}
				maxlength="18"
			/>
		</label>
		<div class="text-right mt-20">
			<button
				class="btn mr-5"
				type="button"
				disabled={submiting}
				on:click={() => dispatch("cancel")}>Cancel</button
			>
			<button class="btn btn-primary" type="submit" disabled={submiting}>
				{#await promise}
					<i class="fas fa-circle-notch fa-spin" />
				{:then}
					Done
				{/await}
			</button>
		</div>
	</form>
</div>
