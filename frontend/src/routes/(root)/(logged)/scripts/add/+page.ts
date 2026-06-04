import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { get } from 'svelte/store'
import { userStore } from '$lib/stores'
import type { PageLoad } from './$types'

// Redirect at the load phase, before the +page.svelte ever mounts. The
// onMount-then-goto version this replaces painted a blank component for
// one frame before navigating, hence the white flash; doing the work in
// `load` lets SvelteKit cancel the route transition and head straight
// for the edit page.
//
// `userStore` is populated client-side by the (logged) layout, so by
// the time this load runs (always after layout setup) it usually has
// the real username. The `'me'` fallback is the same one the original
// onMount used — keeps behavior identical for the rare race where the
// store hasn't filled yet.

export const prerender = false

export const load: PageLoad = ({ url }) => {
	const username = get(userStore)?.username ?? 'me'
	const uuid = crypto.randomUUID()
	const params = new URLSearchParams(url.searchParams)
	params.set('new_draft', 'true')
	redirect(307, `${base}/scripts/edit/u/${username}/draft_${uuid}?${params.toString()}`)
}
