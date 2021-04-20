<script>
	import { login } from "./request"
	import { fly } from "svelte/transition"
	export let logged = false
	import { devices, getDevices } from "./store.js"

	let promise = login(),
		key,
		loggingin = false,
		failed = false

	promise.then(res => {
		if (res.success) success(res)
		console.debug(getDevices())
	})

	function success(res) {
		logged = res.success
		devices.set(Array.isArray(res.devices) ? res.devices : [])
	}

	async function submit() {
		loggingin = true
		const res = await login(key)
		if (res.success) {
			success(res)
		} else {
			failed = true
		}
		setTimeout(() => {
			loggingin = false
		}, 300)
	}
</script>

<div class="h-full d-flex justify-content-center align-items-center">
	{#await promise}
		<i class="fad fa-spinner-third fa-spin fa-3x" />
	{:then}
		{#if !logged}
			<div class="card w-500">
				{#if failed}
					<div class="alert alert-danger my-5" transition:fly>Wrong token</div>
				{/if}
				<h4>Login</h4>
				<form on:submit|preventDefault={submit}>
					<label class="w-full">
						Token
						<input
							type="password"
							bind:value={key}
							class="form-control"
							required="required"
						/>
					</label>
					<button class="btn mt-5 btn-primary btn-block" type="submit" disabled={loggingin}>
						{#if loggingin}
							<i class="fas fa-circle-notch fa-spin" />
						{/if}
						Login
					</button>
				</form>
			</div>
		{/if}
	{:catch}
		<div class="alert alert-danger my-5" transition:fly>Server offline</div>
	{/await}
</div>
