import type { Reroute } from '@sveltejs/kit'

/** @type {import('@sveltejs/kit').Handle} */
export async function handle({ event, resolve }) {
	const response = await resolve(event, {
		ssr: false
	})

	return response
}

export const reroute: Reroute = ({ url }) => {
	const match = url.pathname.match(/^\/w\/([^/]+)(\/.*)?$/)
	if (match) {
		return match[2] || '/'
	}
	return url.pathname
}
