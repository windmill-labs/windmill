import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { getUsernameForNamespace } from '$lib/userNamespace'
import type { PageLoad } from './$types'

// See /scripts/add/+page.ts for the rationale — redirect at load time
// to avoid mounting a blank route component and flashing white.

export const prerender = false

export const load: PageLoad = ({ url }) => {
	const username = getUsernameForNamespace()
	const uuid = crypto.randomUUID()
	const params = new URLSearchParams(url.searchParams)
	params.set('new_draft', 'true')
	redirect(307, `${base}/flows/edit/u/${username}/draft_${uuid}?${params.toString()}`)
}
