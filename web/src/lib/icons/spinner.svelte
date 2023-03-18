<div class="loader" />

<style>
	.loader {
		font-size: inherit;
		border: 2px solid #414141;
		border-radius: 50%;
		border-top: 2px solid #e7e7e7;
		width: 1em;
		height: 1em;
		animation: spin 2s linear infinite;
		aspect-ratio: 1;
		align-self: center;
		margin: 0 5px;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
