import { makeDraftAddLoad } from '$lib/draftAddRedirect'
import type { PageLoad } from './$types'

export const prerender = false

export const load: PageLoad = makeDraftAddLoad('apps/edit')
