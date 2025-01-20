import { sveltekit } from '@sveltejs/kit/vite'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import circleDependency from 'vite-plugin-circular-dependency'
// import mkcert from 'vite-plugin-mkcert'
import importMetaUrlPlugin from '@windmill-labs/esbuild-import-meta-url-plugin'

const file = fileURLToPath(new URL('package.json', import.meta.url))
const json = readFileSync(file, 'utf8')
const version = JSON.parse(json)

/** @type {import('vite').UserConfig} */
const config = {
	server: {
		https: false,
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
		include: ['highlight.js', 'highlight.js/lib/core', 'monaco-vim'],
		exclude: [
			'@codingame/monaco-vscode-standalone-typescript-language-features',
			'@codingame/monaco-vscode-standalone-languages'
		],
		esbuildOptions: {
			plugins: [importMetaUrlPlugin]
		}
	},
	resolve: {
		alias: {
			path: 'path-browserify',
			'monaco-editor/esm/vs/editor/contrib/hover/browser/hover':
				'vscode/vscode/vs/editor/contrib/hover/browser/hoverContribution'
		},
		dedupe: ['vscode', 'monaco-editor']
	},
	worker: {
		format: 'es'
	},
	assetsInclude: ['**/*.wasm']
}

export default config
