import preprocess from 'svelte-preprocess'
import adapter from '@sveltejs/adapter-static'

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
		adapter: process.env.CLOUDFLARE
			? adapter({ pages: 'build', assets: 'build', fallback: 'index.html', precompress: false })
			: adapter({
					// default options are shown
					pages: 'build',
					assets: 'build',
					fallback: '200.html'
			  }),
		prerender: { entries: [] }
	}
}

export default config
