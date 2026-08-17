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
		// Same for every build of one revision (SvelteKit's Date.now() default is not, so
		// the per-architecture images disagreed on content-hashed asset filenames and
		// mixed-arch clusters 404ed on each other's chunks), and different between
		// revisions (SvelteKit only full-page reloads on a missing chunk if this changed).
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
