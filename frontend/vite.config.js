import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import mkcert from 'vite-plugin-mkcert'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

let plugin = {
	name: 'configure-response-headers',
	configureServer: (server) => {
		server.middlewares.use((_req, res, next) => {
			res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
			res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp')
			next()
		})
	}
}

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		https: process.env.HTTPS === 'true',
		allowedHosts: [
			'localhost',
			'127.0.0.1',
			'0.0.0.0',
			'rubendev.wimill.xyz',
			'windmill.xyz',
			'app.windmill.xyz',
			'public.windmill.xyz'
		],
		port: 3000,
		cors: { origin: '*' },
		proxy: {
			'^/\\.well-known/.*': {
				target: process.env.REMOTE ?? 'https://app.windmill.dev',
				changeOrigin: true,
				cookieDomainRewrite: 'localhost'
			},
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
			'^/ws_debug/.*': {
				target: process.env.REMOTE_DEBUG ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true,
				rewrite: (path) => path.replace(/^\/ws_debug/, '')
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
	preview: { port: 3001 },
	plugins: [sveltekit(), ...(process.env.HTTPS === 'true' ? [mkcert()] : []), plugin],
	define: { __pkg__: version },
	optimizeDeps: {
		include: ['highlight.js', 'highlight.js/lib/core', 'monaco-vim'],
		exclude: [
			'@codingame/monaco-vscode-standalone-typescript-language-features',
			'@codingame/monaco-vscode-standalone-languages',
			'windmill-client'
		]
	},
	worker: { format: 'es' },
	resolve: {
		alias: {
			path: 'path-browserify',
			'monaco-editor/esm/vs/editor/contrib/hover/browser/hover':
				'monaco-editor/esm/vs/editor/contrib/hover/browser/hoverContribution'
		},
		dedupe: ['vscode', 'monaco-editor']
	},
	assetsInclude: ['**/*.wasm'],
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.js',
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
					setupFiles: ['src/lib/test-setup.ts']
				}
			}
		]
	}
}

export default config
