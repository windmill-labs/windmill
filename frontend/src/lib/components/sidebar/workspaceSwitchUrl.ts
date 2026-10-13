import { page } from '$app/state'
import { goto } from '$lib/navigation'

// Post-workspace-switch URL fixup shared by the sidebar workspace pickers.
// Item-scoped pages would show a wrong-workspace (or missing) item after a
// switch — go home instead. Otherwise, really rewrite the params in the address
// bar that name a workspace (mutating page.url is a no-op there): left stale,
// they get re-applied on reload or when exiting session mode restores the route,
// silently switching the workspace back.
//
// `workspace_id` names the workspace a page acts on (the compare page's pair) and
// `workspace` the one the app is pointed at; a switch moves both. `target`, the
// compare page's one-off migration destination, is dropped instead of rewritten:
// it was chosen for the workspace being left, and keeping it could point the new
// one at itself.
const WORKSPACE_PARAMS = ['workspace', 'workspace_id']
const STALE_AFTER_SWITCH = ['target']
const EDIT_PAGES = [
	'/scripts/edit/',
	'/flows/edit/',
	'/apps/edit/',
	'/apps_raw/edit/',
	'/scripts/get/',
	'/flows/get/',
	'/apps/get/',
	'/apps_raw/get/'
]

export async function fixupUrlAfterWorkspaceSwitch(
	id: string,
	// Decided by the caller so the rule stays with the picker that has the context
	// for it — a picker rendered outside the sidebar drives its own page and must
	// not be navigated away from.
	opts?: { landOnHome?: boolean }
): Promise<void> {
	if (EDIT_PAGES.some((p) => page.route.id?.includes(p) ?? false) || opts?.landOnHome) {
		await goto('/')
		return
	}
	// `window.location`, never `page.url`: a param written by a previous shallow update
	// is in the address bar but not in `page.url`, so rebuilding from it would undo that
	// write (see the note on `setQuery` in $lib/navigation).
	const url = new URL(window.location.href)
	let changed = false
	for (const key of WORKSPACE_PARAMS) {
		if (url.searchParams.get(key) === id) continue
		if (!url.searchParams.has(key)) continue
		url.searchParams.set(key, id)
		changed = true
	}
	for (const key of STALE_AFTER_SWITCH) {
		if (!url.searchParams.has(key)) continue
		url.searchParams.delete(key)
		changed = true
	}
	if (!changed) return
	// `goto` rather than `replaceState`: a page reading these params through `page.url`
	// (the compare page derives the workspace it compares from them) sees nothing when
	// they are written shallowly, since SvelteKit refreshes `page.url` only on a
	// router-driven navigation.
	await goto(`${url.pathname}${url.search}${url.hash}`, {
		replaceState: true,
		noScroll: true,
		keepFocus: true
	})
}
