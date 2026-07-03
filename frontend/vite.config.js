import { sveltekit } from '@sveltejs/kit/vite'
import { existsSync, readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import mkcert from 'vite-plugin-mkcert'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

// The postinstall downloads the pinned UI Builder artifact into static/ui_builder,
// which SvelteKit serves at /ui_builder. Serve that directly; only proxy to a
// live UI Builder dev server on :4000 when the bundle is absent (mirrors the
// backend's static-vs-:4000 fallback). Delete static/ui_builder to develop the
// builder against :4000.
const uiBuilderStaticPresent = existsSync(
	fileURLToPath(new URL('static/ui_builder/app-preview.html', import.meta.url))
)

const remoteUrl =
	process.env.REMOTE ??
	(process.env.BACKEND_PORT
		? `http://localhost:${process.env.BACKEND_PORT}`
		: 'https://app.windmill.dev/')

const cookieDomain = process.env.ISOLATE_DEV_AUTH === '1' ? '' : 'localhost'

// `enforce: 'pre'` so these headers are set before SvelteKit's sirv static
// handler serves `static/` files and ends the response without calling next().
let plugin = {
	name: 'configure-response-headers',
	enforce: 'pre',
	configureServer: (server) => {
		server.middlewares.use((_req, res, next) => {
			res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
			res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp')
			res.setHeader('Cross-Origin-Resource-Policy', 'cross-origin')
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
		port: parseInt(process.env.FRONTEND_PORT) || 3000,
		cors: { origin: '*' },
		proxy: {
			'^/\\.well-known/.*': {
				target: remoteUrl,
				changeOrigin: true,
				cookieDomainRewrite: cookieDomain
			},
			'^/api/w/[^/]+/s3_proxy/.*': {
				target: remoteUrl,
				changeOrigin: false, // Important for signature to be correct
				cookieDomainRewrite: cookieDomain,
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
				cookieDomainRewrite: cookieDomain
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
			...(uiBuilderStaticPresent
				? {}
				: {
						'^/ui_builder/.*': {
							target: 'http://localhost:4000',
							changeOrigin: true,
							headers: {
								'Cross-Origin-Opener-Policy': 'same-origin',
								'Cross-Origin-Embedder-Policy': 'require-corp',
								'Cross-Origin-Resource-Policy': 'cross-origin'
							}
						}
					})
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
