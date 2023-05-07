import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		port: 3000,
		proxy: {
			'^/api/.*': {
				target: process.env.REMOTE ?? 'https://app.windmill.dev/',
				changeOrigin: true,
				cookieDomainRewrite: 'localhost'
			},
			// Proxying websockets or socket.io: ws://localhost:5173/socket.io -> ws://localhost:5174/socket.io
			'^/ws/.*': {
				target: process.env.REMOTE_LSP ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true
			}
		}
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
