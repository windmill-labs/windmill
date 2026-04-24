import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import mkcert from 'vite-plugin-mkcert'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

const remoteUrl =
	process.env.REMOTE ??
	(process.env.BACKEND_PORT
		? `http://localhost:${process.env.BACKEND_PORT}`
		: 'https://app.windmill.dev/')

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
			'public.windmill.xyz',
			'hugo.ngrok.pro'
		],
		port: parseInt(process.env.FRONTEND_PORT) || 3000,
		cors: { origin: '*' },
		proxy: {
			'^/\\.well-known/.*': {
				target: remoteUrl,
				changeOrigin: true,
				cookieDomainRewrite: 'localhost'
			},
			'^/api/w/[^/]+/s3_proxy/.*': {
				target: remoteUrl,
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
				target: remoteUrl,
				changeOrigin: true,
				cookieDomainRewrite: 'localhost'
			},
			'^/ws/.*': {
				target: process.env.REMOTE_LSP ?? process.env.REMOTE_EXTRA ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true
			},
			'^/ws_mp/.*': {
				target: process.env.REMOTE_MP ?? process.env.REMOTE_EXTRA ?? 'https://app.windmill.dev',
				changeOrigin: true,
				ws: true
			},
			'^/ws_debug/.*': {
				target: process.env.REMOTE_DEBUG ?? process.env.REMOTE_EXTRA ?? 'https://app.windmill.dev',
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
	build: {
		rollupOptions: {
			output: {
				manualChunks(id) {
					if (id.includes('node_modules')) {
						if (id.includes('monaco-editor') || id.includes('@codingame/monaco-vscode')) {
							return 'vendor-monaco'
						}
						if (id.includes('/openai/') || id.includes('/@anthropic-ai/')) {
							return 'vendor-ai'
						}
						if (id.includes('@xyflow/svelte') || id.includes('d3-dag')) {
							return 'vendor-flow'
						}
						if (id.includes('/yjs/') || id.includes('y-websocket') || id.includes('y-monaco')) {
							return 'vendor-collab'
						}
						if (id.includes('ag-grid') || id.includes('ag-charts')) {
							return 'vendor-ag'
						}
						if (id.includes('pdfjs-dist')) {
							return 'vendor-pdf'
						}
						if (
							id.includes('svelte-exmarkdown') ||
							id.includes('/parse5/') ||
							id.includes('/rehype-') ||
							id.includes('/remark-') ||
							id.includes('/mdast-') ||
							id.includes('/micromark')
						) {
							return 'vendor-markdown'
						}
						if (id.includes('quicktype-core')) {
							return 'vendor-quicktype'
						}
						if (id.includes('/quill/')) {
							return 'vendor-quill'
						}
						if (id.includes('/chart.js/') || id.includes('chartjs-')) {
							return 'vendor-chart'
						}
					}
				}
			}
		}
	},
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
