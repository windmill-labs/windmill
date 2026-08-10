import { page } from '$app/state'
import { buildFilterUrl } from '$lib/navigation'
import { pageHref, stripBase } from './previewRouter'
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

/**
 * "Open in AI session" source for the edit drawer of a workspace list page — the
 * trigger lists, schedules, resources, variables. The session opens the page
 * itself with the drawer deep-linked, since none of these are editable items the
 * preview can host.
 *
 * Returns undefined (so the button renders nothing) unless the page's own route
 * is the one on screen: the very same drawers open inside script/flow editors and
 * pickers, where the enclosing editor already carries its own entry point.
 *
 * `anchor` is the row in that page's deep-link convention — a bare path for most,
 * `/resource/<path>` for resources — which is what its `#<hash>` handler reads
 * back. Empty while an item is still being created.
 */
export function pageDrawerSessionSource(
	pagePath: string,
	anchor: string | undefined,
	workspaceId: string | undefined
): OpenInSessionSource | undefined {
	if (!anchor || stripBase(page.url.pathname) !== pagePath) return undefined
	const href = pageHref(buildFilterUrl(pagePath, {}, { hash: anchor }))
	return { page: () => href, workspaceId }
}
