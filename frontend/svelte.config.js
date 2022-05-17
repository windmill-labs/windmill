import preprocess from 'svelte-preprocess'
import adapter from '@sveltejs/adapter-static'
const ppath = process.env.PREVIEW_PATH

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';

const file = fileURLToPath(new URL('package.json', import.meta.url));
const json = readFileSync(file, 'utf8');
const version = JSON.parse(json)



/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://github.com/sveltejs/svelte-preprocess
	// for more information about preprocessors
	preprocess: [
		preprocess({
			postcss: true
		})
	],

	kit: {
		paths: {
			base: ppath ? ppath : ''
		},
		vite: {
			define: {
				'__pkg__': version
			},
			ssr: {
				noExternal: ['dayjs']
			},
			optimizeDeps: {
				include: [
					'highlight.js',
					'highlight.js/lib/core',
					'@codingame/monaco-languageclient/lib/vscode-compatibility'
				]
			},
			resolve: {
				alias: [
					{
						find: 'vscode',
						replacement: '@codingame/monaco-languageclient/lib/vscode-compatibility'
					}
				]
			}
		},
		adapter: adapter({
			// default options are shown
			pages: 'build',
			assets: 'build',
			fallback: '200.html'
		}),
		prerender: {
			enabled: true
		}
	}
}

export default config
