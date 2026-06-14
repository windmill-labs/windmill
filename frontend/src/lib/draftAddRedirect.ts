import { redirect } from '@sveltejs/kit'
import { base } from '$app/paths'
import { getUsernameForNamespace } from '$lib/userNamespace'
import { randomUUID } from '$lib/utils/uuid'

/**
 * Shared `load` for every `/{scripts,flows,apps,apps_raw}/add` route. Doing the
 * redirect in `load` (not `onMount`) avoids painting a blank frame first.
 *
 * Mints a fresh `u/<username>/draft_<uuid>` path and 307s to
 * `{base}/{editPrefix}/<path>?new_draft=true&<existing-params>`. `new_draft=true`
 * tells the edit route to seed an empty editor instead of 404-ing. The hash is
 * carried over too — fork / handler-template buttons encode a payload into it
 * that the edit route's `new_draft` branch consumes. `randomUUID` not
 * `crypto.randomUUID` (WebCrypto is absent on non-secure origins).
 */
export function makeDraftAddLoad(editPrefix: string) {
	return ({ url }: { url: URL }) => {
		const username = getUsernameForNamespace()
		// Underscores not dashes — path segments are `[a-zA-Z0-9_]` words and
		// downstream consumers treat `-` as foreign.
		const uuid = randomUUID().replaceAll('-', '_')
		const params = new URLSearchParams(url.searchParams)
		params.set('new_draft', 'true')
		// `url.hash` is unavailable in `load`; read `window.location` instead
		// (safe — the app is SPA-only, `ssr = false`). On client-side nav it
		// points at the PREVIOUS page, but every hash-payload producer arrives
		// as a full page load, so the hash is correct.
		const hash = typeof window !== 'undefined' ? window.location.hash : ''
		redirect(307, `${base}/${editPrefix}/u/${username}/draft_${uuid}?${params.toString()}${hash}`)
	}
}
