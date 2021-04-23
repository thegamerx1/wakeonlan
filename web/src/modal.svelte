<script>
	import { devices, getDevices } from "./store"
	import { save } from "./request"
	import { fly } from "svelte/transition"

	export let show = false

	$: if (show) {
		input.focus()
	}

	const toCheck = ["name", "mac", "host"]
	let data, error, submiting, input
	reset()

	function onSubmit() {
		let current = getDevices()
		let error = false
		submiting = true
		toCheck.forEach(check => {
			for (var i = 0; i < current.length; i++) {
				if (error) break
				if (current[i][check] == data[check]) error = true
			}
		})

		if (error) return

		save([...$devices, data]).then(data => {
			devices.set(data.devices)
			show = submiting = false
		})
		reset()
	}

	function reset() {
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
			{#if error}
				<div class="alert alert-danger" role="alert">Duplicate data found</div>
			{/if}
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
						placeholder="laptop or 192.168.1.75"
						disabled={submiting}
					/>
				</label>
				<div class="text-right mt-20">
					<button
						class="btn mr-5"
						on:click={() => (show = false)}
						type="button"
						disabled={submiting}>Close</button
					>
					<button class="btn btn-primary" type="submit" disabled={submiting}>Add</button>
				</div>
			</form>
		</div>
	</div>
</div>
