import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			preprocess: [vitePreprocess()],
			adapter: adapter({ pages: 'build', assets: 'build', strict: true })
		})
	],
	server: {
		proxy: {
			'/ws': {
				target: 'ws://localhost:9000',
				ws: true
			}
		}
	}
});
