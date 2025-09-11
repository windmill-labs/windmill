import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import mkcert from 'vite-plugin-mkcert'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		https: process.env.HTTPS === 'true',
		allowedHosts: ['localhost', '127.0.0.1', '0.0.0.0', 'rubendev.wimill.xyz'],
		port: 3000,
		proxy: {
			'^/api/w/[^/]+/s3_proxy/.*': {
				target: process.env.REMOTE ?? 'https://app.windmill.dev/',
				changeOrigin: false, // Important for signature to be correct
				cookieDomainRewrite: 'localhost',
				configure: (proxy, options) => {
					proxy.on('proxyReq', (proxyReq, req, res) => {
						// Prevent collapsing slashes during URL normalization
						const originalPath = req.url
						proxyReq.path = originalPath
					})
				}
			},
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
	plugins: [sveltekit(), ...(process.env.HTTPS === 'true' ? [mkcert()] : [])],
	define: {
		__pkg__: version
	},
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core', 'monaco-vim', 'monaco-editor-wrapper'],
		exclude: [
			'@codingame/monaco-vscode-standalone-typescript-language-features',
			'@codingame/monaco-vscode-standalone-languages'
		],
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
