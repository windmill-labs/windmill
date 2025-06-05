import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import mkcert from 'vite-plugin-mkcert'
import importMetaUrlPlugin from '@windmill-labs/esbuild-import-meta-url-plugin'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		https: process.env.HTTPS === 'true',
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
			},
			'^/ui_builder/.*': {
				target: 'http://localhost:4000',
				changeOrigin: true,
				headers: {
					'Cross-Origin-Opener-Policy': 'same-origin',
					'Cross-Origin-Embedder-Policy': 'require-corp',
					'Cross-Origin-Resource-Policy': 'cross-origin'
				}
			}
		}
	},
	preview: {
		port: 3000
	},
	plugins: [
		nodePolyfills({ include: ['buffer'] }),
		sveltekit(),
		...(process.env.HTTPS === 'true' ? [mkcert()] : [])
	],
	define: {
		__pkg__: version
	},
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core', 'monaco-vim', 'monaco-editor-wrapper'],
		exclude: [
			'@codingame/monaco-vscode-standalone-typescript-language-features',
			'@codingame/monaco-vscode-standalone-languages'
		],
		esbuildOptions: {
			plugins: [importMetaUrlPlugin]
		}
	},
	worker: {
		format: 'es'
	},
	resolve: {
		alias: {
			path: 'path-browserify',
			'monaco-editor/esm/vs/editor/contrib/hover/browser/hover':
				'monaco-editor/esm/vs/editor/contrib/hover/browser/hoverContribution'
		},
		dedupe: ['vscode', 'monaco-editor']
	},
	assetsInclude: ['**/*.wasm']
}

export default config
