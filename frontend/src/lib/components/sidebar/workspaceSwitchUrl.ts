import { page } from '$app/state'
import { replaceState } from '$app/navigation'
import { goto } from '$lib/navigation'

// Post-workspace-switch URL fixup shared by the sidebar workspace pickers.
// Item-scoped pages would show a wrong-workspace (or missing) item after a
// switch — go home instead. Otherwise, really rewrite a ?workspace= param in
// the address bar (mutating page.url is a no-op there): left stale, it gets
// re-applied on reload or when exiting session mode restores the route,
// silently switching the workspace back.
const EDIT_PAGES = [
	'/scripts/edit/',
	'/flows/edit/',
	'/apps/edit/',
	'/scripts/get/',
	'/flows/get/',
	'/apps/get/'
]

export async function fixupUrlAfterWorkspaceSwitch(id: string): Promise<void> {
	if (EDIT_PAGES.some((p) => page.route.id?.includes(p) ?? false)) {
		await goto('/')
	} else if (page.url.searchParams.get('workspace')) {
		const url = new URL(window.location.href)
		url.searchParams.set('workspace', id)
		replaceState(url, page.state)
	}
}
