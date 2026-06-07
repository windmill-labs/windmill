import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { getUsernameForNamespace } from '$lib/userNamespace'
import type { PageLoad } from './$types'

// Redirect at the load phase, before the +page.svelte ever mounts. The
// onMount-then-goto version this replaces painted a blank component for
// one frame before navigating, hence the white flash; doing the work in
// `load` lets SvelteKit cancel the route transition and head straight
// for the edit page.

export const prerender = false

export const load: PageLoad = ({ url }) => {
	const username = getUsernameForNamespace()
	const uuid = crypto.randomUUID()
	const params = new URLSearchParams(url.searchParams)
	params.set('new_draft', 'true')
	redirect(307, `${base}/scripts/edit/u/${username}/draft_${uuid}?${params.toString()}`)
}
