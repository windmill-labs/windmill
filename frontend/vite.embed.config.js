// import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { svelte } from '@sveltejs/vite-plugin-svelte'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		port: 3000
	},
	preview: {
		port: 3000
	},
	plugins: [svelte()],
	define: {
		__pkg__: version
	},
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core']
	},
	resolve: {
		alias: {
			path: 'path-browserify',
			$lib: ['/src/lib'],
			'$lib/*': ['/src/lib/*'],
			'$app/navigation': ['/src/lib/fakesvelte.js'],
			'$app/stores': ['/src/lib/fakesvelte.js'],
			'$app/environment': ['/src/lib/fakesvelte.js']
		}
	},
	build: {
		lib: {
			// Could also be a dictionary or array of multiple entry points
			entry: ['src/embed.js'],
			name: 'AppViewer',
			// the proper extensions will be added
			fileName: 'appviewer',
			formats: ['iife']
		}
		// rollupOptions: {
		// 	external: ['highlight.js'],
		// 	input: 'src/embed.js',
		// 	output: {
		// 		format: 'esm',
		// 		name: 'app',
		// 		// We output it to public. This way, our svelte kit
		// 		// app will also host the web components.
		// 		dir: 'build'
		// 		// file: 'build/app.js'
		// 	}
		// }
	}
}

export default config
