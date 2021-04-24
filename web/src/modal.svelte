<script>
	import { add, toCheck } from "./store"

	export let show = false
	let data, error, submiting, input

	$: if (show) {
		reset()
		input.focus()
	}
	reset()

	function onSubmit() {
		error = false
		submiting = true
		add(data)
			.then(() => {
				show = false
			})
			.catch(e => {
				error = true
				submiting = false
			})
	}

	function reset() {
		error = submiting = false
		data = {}
		toCheck.forEach(check => {
			data[check] = ""
		})
	}
</script>

<div
	class="modal"
	class:show
	role="dialog"
	data-overlay-dismissal-disabled="true"
	data-esc-dismissal-disabled="true"
>
	<div class="modal-dialog" role="document">
		<div class="modal-content">
			<h5 class="modal-title">Add device</h5>
			<form on:submit|preventDefault={onSubmit}>
				{#if error}
					<div class="invalid-feedback">A device with that hostname already exists!</div>
				{/if}
				<label class="w-full"
					>Name
					<input
						type="text"
						bind:this={input}
						bind:value={data.name}
						class="form-control"
						placeholder="Name"
						disabled={submiting}
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
					/>
				</label>
				<div class="text-right mt-20">
					<button
						class="btn mr-5"
						type="button"
						disabled={submiting}
						on:click={() => (show = false)}>Close</button
					>
					<button class="btn btn-primary" type="submit" disabled={submiting}>Add</button>
				</div>
			</form>
		</div>
	</div>
</div>
