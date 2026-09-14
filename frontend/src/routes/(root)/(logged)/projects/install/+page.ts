import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import type { PageLoad } from './$types'

/**
 * `/projects/install?hub=<slug>` was where the hub's "Add to workspace" button
 * pointed before the import wizard existed. Hubs upgrade on their own schedule —
 * a self-hosted one may keep sending people here for a long time — so the old
 * entry point forwards to the wizard rather than 404ing, query string intact.
 */
export const load: PageLoad = ({ url }) => {
	redirect(307, `${base}/projects/import${url.search}`)
}
