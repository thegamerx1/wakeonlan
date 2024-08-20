// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
	}

	type Device = {
		name: string;
		mac: string;
		host: string;
		api_key: string | null;
	};
}

export {};
