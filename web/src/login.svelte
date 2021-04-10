<script>
	import { login, status } from "./request"
	import { fly } from "svelte/transition"
	import { devices } from "./store.js"
	import { onMount } from "svelte"

	let promise,
		key,
		loggingin = false,
		failed = false,
		show = false

	onMount(() => {
		promise = login().then(data => {
			if (data.success) {
				success(data)
			} else {
				show = true
			}
		})
	})

	function success(data) {
		failed = false
		devices.set(Array.isArray(data.devices) ? data.devices : [])
	}

	async function submit() {
		loggingin = true
		const data = await login(key)
		if (data.success) {
			localStorage.setItem("token", key)
			success(data)
		} else {
			failed = true
		}
		setTimeout(() => {
			loggingin = false
		}, 300)
	}
</script>

{#if show}
	<div class="card w-full w-sm-500" transition:fly>
		{#if failed}
			<div class="alert alert-danger my-5" transition:fly>Wrong token</div>
		{/if}
		<h4>Login</h4>
		<form on:submit|preventDefault={submit}>
			<label class="w-full">
				Token
				<input type="password" bind:value={key} class="form-control" required="required" />
			</label>
			<button class="btn mt-5 btn-primary btn-block" type="submit" disabled={loggingin}>
				{#if loggingin}
					<i class="fad fa-spinner-third fa-spin" />
				{/if}
				Login
			</button>
		</form>
	</div>
{/if}
