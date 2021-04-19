<script>
	import { devices, getDevices } from "./store.js"

	const toCheck = ["name", "mac"]

	export let show = false
	let data, error

	reset()

	function onSubmit() {
		let current = getDevices()
		let error = false
		toCheck.forEach(check => {
			for (var i = 0; i < current.length; i++) {
				if (error) break
				if (current[i][check] == data[check]) error = true
			}
		})

		if (error) return

		devices.update(arr => [...arr, data])
		show = false
		reset()
	}

	function reset() {
		data = { name: "", mac: "" }
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
			{#if error}
				<div class="alert alert-danger" role="alert">Duplicate data found</div>
			{/if}
			<form on:submit|preventDefault={onSubmit}>
				<label class="w-full"
					>Name
					<input
						type="text"
						bind:value={data.name}
						class="form-control"
						placeholder="Name"
						required="required"
					/>
				</label>
				<label class="w-full"
					>MAC Address
					<input
						type="text"
						bind:value={data.mac}
						class="form-control"
						placeholder="00:00:00:00:00"
						required="required"
					/>
				</label>
				<div class="text-right mt-20">
					<button class="btn mr-5" on:click={() => (show = false)} type="button">Close</button>
					<button class="btn btn-primary" type="submit">Add</button>
				</div>
			</form>
		</div>
	</div>
</div>
