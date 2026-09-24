import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { getUsernameForNamespace } from '$lib/userNamespace'
import { random_adj } from '$lib/components/random_positive_adjetive'
import type { PageLoad } from './$types'

export const prerender = false

// A readable name rather than `mintDraftPath`'s `draft_<uuid>`: an agent's draft is stored at the
// path it is opened on, so this is the name it shows under until its first deploy.
export const load: PageLoad = () => {
	redirect(
		307,
		`${base}/agents/edit/u/${getUsernameForNamespace()}/${random_adj()}_agent?new_draft=true`
	)
}
