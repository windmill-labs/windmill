import preprocess from 'svelte-preprocess'
import adapter from '@sveltejs/adapter-static'
import { preprocessMeltUI, sequence } from '@melt-ui/pp'
import { readFileSync } from 'fs'
import { fileURLToPath } from 'url'

const pkg = JSON.parse(
	readFileSync(fileURLToPath(new URL('package.json', import.meta.url)), 'utf8')
)

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://github.com/sveltejs/svelte-preprocess
	// for more information about preprocessors
	preprocess: sequence([
		preprocess({
			postcss: true
		}),
		preprocessMeltUI()
	]),

	kit: {
		adapter:
			process.env.CLOUDFLARE || process.env.NOCATCHALL
				? adapter({ pages: 'build', assets: 'build', fallback: 'index.html', precompress: false })
				: adapter({
						// default options are shown
						pages: 'build',
						assets: 'build',
						fallback: '200.html'
					}),
		// Must stay deterministic: SvelteKit defaults this to Date.now(), which differs
		// between the per-architecture builds of the same commit and cascades into
		// different content-hashed chunk filenames. Those assets are embedded in the
		// binary, so mixed-arch clusters would 404 on each other's chunks.
		version: { name: process.env.WM_BUILD_VERSION || pkg.version },
		prerender: { entries: [] },
		paths: {
			base: process.env.VITE_BASE_URL ?? ''
		},
		alias: {
			$system_prompts: '../system_prompts/auto-generated',
			$oauth_connect_registry: '../backend/oauth_connect.json'
		}
	},

	vitePlugin: {
		inspector: true
	}
}

export default config
