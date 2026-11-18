import { get } from 'svelte/store'
import { userStore } from '$lib/stores'

/**
 * Workspace username for the authed user, suitable as the `u/{X}/...`
 * namespace in editor draft paths.
 *
 * The `/add` → `/edit/u/{username}/draft_{uuid}` redirects in each
 * editor's `+page.ts` run during SvelteKit's load phase — BEFORE the
 * (logged) layout's async `getUserExt` populates `userStore`. Reading
 * `get(userStore)?.username` from there returns `undefined` on every
 * fresh nav, falling back to the `'me'` placeholder and producing
 * `u/me/draft_{uuid}` paths instead of the real namespace.
 *
 * The layout writes `username` to `localStorage` on every successful
 * `getUserExt`, so this helper:
 *   1. Reads `userStore` first (live value when it IS populated, e.g.
 *      same-tab re-navs after the first load).
 *   2. Falls back to `localStorage` (warm-cache from the previous
 *      session — covers the load-phase race on cold reload).
 *   3. Falls back to `'me'` (true first-ever load with no session yet).
 */
export function getUsernameForNamespace(): string {
	const live = get(userStore)?.username
	if (live) return live
	try {
		const cached = localStorage.getItem('username')
		if (cached) return cached
	} catch {}
	return 'me'
}
