import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { mintDraftPath } from '$lib/mintDraftPath'

/**
 * Shared `load` for every `/{scripts,flows,apps,apps_raw}/add` route. Doing the
 * redirect in `load` (not `onMount`) avoids painting a blank frame first.
 *
 * Mints a fresh `u/<username>/draft_<uuid>` path (via the shared `mintDraftPath`,
 * so the SDK builder wrappers stay in lockstep) and 307s to
 * `{base}/{editPrefix}/<path>?new_draft=true&<existing-params>`. `new_draft=true`
 * tells the edit route to seed an empty editor instead of 404-ing. The hash is
 * carried over too — fork / handler-template buttons encode a payload into it
 * that the edit route's `new_draft` branch consumes.
 */
export function makeDraftAddLoad(editPrefix: string) {
	return ({ url }: { url: URL }) => {
		const path = mintDraftPath()
		const params = new URLSearchParams(url.searchParams)
		params.set('new_draft', 'true')
		// `url.hash` is unavailable in `load`; read `window.location` instead
		// (safe — the app is SPA-only, `ssr = false`). On client-side nav it
		// points at the PREVIOUS page, but every hash-payload producer arrives
		// as a full page load, so the hash is correct.
		const hash = typeof window !== 'undefined' ? window.location.hash : ''
		redirect(307, `${base}/${editPrefix}/${path}?${params.toString()}${hash}`)
	}
}
