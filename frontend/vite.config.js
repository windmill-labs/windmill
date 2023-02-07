import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'

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
	plugins: [sveltekit()],
	define: {
		__pkg__: version
	},
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core']
	},
	resolve: {
		alias: {
			path: 'path-browserify'
		}
	}
}

export default config
