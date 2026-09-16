import { base } from '$lib/base'

// Href for a workspace-switch link in the sidebar WorkspaceMenu: stay on the
// current path and just swap the `workspace` query param, so a modifier/middle
// click (open in new tab, which bypasses the onClick fast-path) lands on the
// same page — session mode included — in the *clicked* workspace. A session
// named in the URL is kept only for same-family targets: a cross-family tab
// must not open a foreign family's chat, so it lands on the sessions page with
// nothing selected instead.
export function workspaceMenuHref(args: {
	pathname: string
	searchParams: URLSearchParams
	id: string
	// Whether `id` belongs to the same workspace family as the active workspace.
	sameFamily?: boolean
	// Land on home instead of the current page — an operator's own pages are
	// granted per workspace, so the one they are on may not be theirs to open in
	// the workspace they are switching into. Drops the current page's params with
	// it: they describe a page the target is not going to show.
	landOnHome?: boolean
}): string {
	if (args.landOnHome) {
		return `${base}/?workspace=${encodeURIComponent(args.id)}`
	}
	const params = new URLSearchParams(args.searchParams)
	params.set('workspace', args.id)
	if (!args.sameFamily) {
		params.delete('session_name')
	}
	return `${args.pathname}?${params.toString()}`
}
