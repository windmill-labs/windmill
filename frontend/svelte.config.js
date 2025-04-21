import preprocess from 'svelte-preprocess'
import adapter from '@sveltejs/adapter-static'
import { preprocessMeltUI, sequence } from '@melt-ui/pp'

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
		prerender: { entries: [] },
		paths: {
			base: process.env.VITE_BASE_URL ?? ''
		}
	}
}

export default config
