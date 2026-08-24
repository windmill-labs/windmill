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

/**
 * Fail the build if the emitted chunks import each other in a cycle.
 *
 * A chunk's imports all evaluate before its own body, so in a cycle one member runs
 * while another has initialized nothing: its `const`/`class` exports read as
 * `undefined` and the app dies on load with "X is not a constructor" behind
 * SvelteKit's default 500 page. The module graph being acyclic is not enough —
 * chunk *grouping* alone can create one. See docs/frontend-import-cycles.md.
 */
function assertAcyclicChunks() {
	return {
		name: 'wm-assert-acyclic-chunks',
		generateBundle(_options, bundle) {
			const imports = new Map()
			for (const [file, chunk] of Object.entries(bundle)) {
				if (chunk.type === 'chunk') imports.set(file, chunk.imports ?? [])
			}
			const state = new Map() // file -> 'visiting' | 'done'
			const describe = (file) => {
				const chunk = bundle[file]
				const ids = chunk.moduleIds ?? Object.keys(chunk.modules ?? {})
				const own = ids.filter((id) => id.includes('/src/')).map((id) => id.split('/src/')[1])
				const shown = own.slice(0, 6).join(', ')
				return `  ${file}\n      ${shown}${own.length > 6 ? `, +${own.length - 6} more` : ''}`
			}
			for (const root of imports.keys()) {
				if (state.get(root)) continue
				const stack = [[root, 0]]
				const path = [root]
				state.set(root, 'visiting')
				while (stack.length) {
					const frame = stack[stack.length - 1]
					const deps = imports.get(frame[0]) ?? []
					if (frame[1] >= deps.length) {
						state.set(path.pop(), 'done')
						stack.pop()
						continue
					}
					const dep = deps[frame[1]++]
					if (state.get(dep) === 'visiting') {
						const cycle = path.slice(path.indexOf(dep)).concat(dep)
						this.error(
							`Cyclic chunk imports — this ships a runtime crash, see docs/frontend-import-cycles.md:\n` +
								cycle.map(describe).join('\n   ->\n')
						)
					}
					if (!state.has(dep) && imports.has(dep)) {
						state.set(dep, 'visiting')
						path.push(dep)
						stack.push([dep, 0])
					}
				}
			}
		}
	}
}

const remoteUrl =
	process.env.REMOTE ??
	(process.env.BACKEND_PORT
		? `http://localhost:${process.env.BACKEND_PORT}`
		: 'https://app.windmill.dev/')

const cookieDomain = process.env.ISOLATE_DEV_AUTH === '1' ? '' : 'localhost'

// Cross-origin isolation headers, scoped to mirror the production predicate —
// see `needs_cross_origin_isolation` in backend/windmill-api/src/static_assets.rs
// for which paths need them and why the raw app viewer must be excluded.
// `enforce: 'pre'` so these headers are set before SvelteKit's sirv static
// handler serves `static/` files and ends the response without calling next().
function needsCrossOriginIsolation(url) {
	const [path, query = ''] = url.split('?')
	return (
		path.startsWith('/apps_raw/edit') ||
		path.startsWith('/apps_raw/add') ||
		path.startsWith('/ui_builder/') ||
		((path.startsWith('/public/') || path.startsWith('/a/')) &&
			new URLSearchParams(query).has('wm_coep'))
	)
}

let plugin = {
	name: 'configure-response-headers',
	enforce: 'pre',
	configureServer: (server) => {
		server.middlewares.use((req, res, next) => {
			if (needsCrossOriginIsolation(req.url ?? '')) {
				res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
				res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp')
			}
			// CORP on everything so dev assets stay loadable as subresources of
			// isolated documents on other dev origins (e.g. 127.0.0.1 vs localhost).
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
	plugins: [
		sveltekit(),
		...(process.env.HTTPS === 'true' ? [mkcert()] : []),
		plugin,
		assertAcyclicChunks()
	],
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
				// src/lib/gen is a self-contained generated client and a leaf of the module
				// graph. Left alone the bundler splits it — index.ts in one chunk, the
				// 12k-line schemas.gen.ts in another beside app code that imports back —
				// which makes the *chunk* graph cyclic where the module graph is not, and
				// modules then evaluate against uninitialized bindings. See
				// docs/frontend-import-cycles.md.
				advancedChunks: { groups: [{ name: 'gen', test: /[\\/]src[\\/]lib[\\/]gen[\\/]/ }] }
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
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}', 'src/**/*.dom.{test,spec}.{js,ts}'],
					setupFiles: ['src/lib/test-setup.ts']
				}
			},
			{
				// `*.dom.test.ts` — for the pure DOM utilities (snapshot serialization,
				// replay sanitization) whose contracts can only be asserted against a
				// real document.
				extends: './vite.config.js',
				test: {
					name: 'dom',
					environment: 'jsdom',
					include: ['src/**/*.dom.{test,spec}.{js,ts}']
				}
			}
		]
	}
}

export default config
