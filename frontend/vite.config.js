import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import circleDependency from 'vite-plugin-circular-dependency'

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
			'^/ws/.*': {
				target: process.env.REMOTE_LSP ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true
			},
			'^/ws_mp/.*': {
				target: process.env.REMOTE_MP ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true
			}
		}
	},
	preview: {
		port: 3000
	},
	plugins: [sveltekit(), circleDependency({ circleImportThrowErr: false })],
	define: {
		__pkg__: version
	},
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core']
	},
	resolve: {
		alias: {
			path: 'path-browserify'
		},
		dedupe: ['monaco-editor', 'vscode']
	},
	assetsInclude: ['**/*.wasm']
}

export default config
