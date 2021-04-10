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
		submiting = true
		add(data)
			.then(data => {
				if (data.success) {
					show = false
				} else {
					error = true
					submiting = false
				}
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
				<label class="w-full"
					>Name
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
				<div class="text-right mt-20">
					<button
						class="btn mr-5"
						type="button"
						disabled={submiting}
						on:click={() => (show = false)}>Close</button
					>
					<button class="btn btn-primary w-100" type="submit" disabled={submiting}>
						{#if submiting}
							<i class="fas fa-circle-notch fa-spin" />
						{:else}
							Add
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
</div>
