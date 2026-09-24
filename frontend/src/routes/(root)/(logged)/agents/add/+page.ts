import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { getUsernameForNamespace } from '$lib/userNamespace'
import { random_adj } from '$lib/components/random_positive_adjetive'
import { randomUUID } from '$lib/utils/uuid'
import type { PageLoad } from './$types'

export const prerender = false

// A readable name rather than `mintDraftPath`'s `draft_<uuid>`: an agent's draft is stored at the
// path it is opened on, so this is the name it shows under until its first deploy. The suffix keeps
// it from landing on an earlier unfinished agent, which the editor would reopen as this one.
export const load: PageLoad = ({ url }) => {
	const suffix = randomUUID().replaceAll('-', '').slice(0, 4)
	const path = `u/${getUsernameForNamespace()}/${random_adj()}_agent_${suffix}`
	const params = new URLSearchParams(url.searchParams)
	params.set('new_draft', 'true')
	redirect(307, `${base}/agents/edit/${path}?${params.toString()}`)
}
