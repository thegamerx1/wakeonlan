<script lang="ts">
	import { login, status } from '$lib/request';
	import { fade, fly } from 'svelte/transition';
	import { devices } from '$lib/store';
	import { goto } from '$app/navigation';
	import Spinner from '$lib/icons/spinner.svelte';

	let promise,
		key = localStorage.getItem('token') ?? '',
		loggingin = false,
		failed = false,
		show = false,
		firsttry = true;

	$: {
		if ($status.connected && firsttry) {
			if (key) {
				firsttry = false;
				promise = login(key).then((data: any) => {
					if (data.success) {
						success(data);
					} else {
						show = true;
					}
				});
			} else {
				show = true;
			}
		}
	}

	function success(data: any) {
		failed = false;
		devices.set(Array.isArray(data.devices) ? data.devices : []);
		goto('/');
	}

	async function submit() {
		loggingin = true;
		const data: any = await login(key);
		if (data.success) {
			localStorage.setItem('token', key);
			success(data);
		} else {
			failed = true;
		}
		setTimeout(() => {
			loggingin = false;
		}, 300);
	}
</script>

<div class="flex justify-center place-items-center h-full">
	{#if $status.connected}
		{#if show}
			<div class="card w-full w-sm-500" transition:fly>
				{#if failed}
					<div class="alert alert-danger my-5" transition:fly>Wrong token</div>
				{/if}
				<h4 class="text-4xl font-bold pb-2 text-center">Login</h4>
				<form on:submit|preventDefault={submit}>
					<label class="w-full">
						Token
						<input type="password" bind:value={key} class="form-control h-16" required />
					</label>
					<button class="btn mt-5 btn-primary btn-block" type="submit" disabled={loggingin}>
						{#if loggingin}
							<Spinner />
						{:else}
							Login
						{/if}
					</button>
				</form>
			</div>
		{/if}
	{:else if $status.error}
		<div
			class="alert my-5 font-size-16 alert-danger flex place-items-center"
			transition:fade={{ duration: 500 }}
		>
			<Spinner />
			<span class="pl-4"> Connection to server failed..</span>
		</div>
	{:else}
		<Spinner />
	{/if}
</div>
