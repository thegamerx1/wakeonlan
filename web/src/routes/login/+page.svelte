<script lang="ts">
	import { login, status } from '$lib/request';
	import { fade, fly } from 'svelte/transition';
	import { goto } from '$app/navigation';
	import Spinner from '$lib/icons/spinner.svelte';
	import { passwordStore } from '$lib/request';

	let password = '',
		loggingin = false,
		failed = false;

	$: {
		if ($status.authenticated) {
			goto('/');
		}
	}

	async function submit() {
		loggingin = true;
		const data = await login(password);
		if (data.success) {
			passwordStore.set(password);
			goto('/');
		} else {
			failed = true;
		}
		setTimeout(() => {
			loggingin = false;
		}, 300);
	}
</script>

<div class="flex justify-center items-center h-full">
	{#if $status.connected}
		<div class="card w-full w-sm-500 place-self-start" transition:fly>
			{#if failed}
				<div class="alert alert-danger my-5" transition:fly>Wrong token</div>
			{/if}
			<h4 class="text-4xl font-bold pb-2 text-center">Login</h4>
			<form on:submit|preventDefault={submit}>
				<label class="w-full">
					Token
					<input
						type="password"
						bind:value={password}
						class="form-control h-16"
						required
						disabled={loggingin}
					/>
				</label>
				<button
					class="btn mt-5 btn-primary btn-block flex items-center justify-center"
					type="submit"
					disabled={loggingin}
				>
					{#if loggingin}
						<Spinner />
					{:else}
						Login
					{/if}
				</button>
			</form>
		</div>
	{:else if $status.error}
		<div
			class="alert my-5 font-size-16 alert-danger flex items-center"
			transition:fade={{ duration: 500 }}
		>
			<Spinner />
			<span class="pl-4"> Connection to server failed..</span>
		</div>
	{:else}
		<Spinner />
	{/if}
</div>
