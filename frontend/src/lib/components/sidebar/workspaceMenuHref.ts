// Href for a workspace-switch link in the sidebar WorkspaceMenu: stay on the
// current path and just swap the `workspace` query param, so a modifier/middle
// click (open in new tab, which bypasses the onClick fast-path) lands on the
// same page — session mode included — in the *clicked* workspace.
export function workspaceMenuHref(args: {
	pathname: string
	searchParams: URLSearchParams
	id: string
}): string {
	const params = new URLSearchParams(args.searchParams)
	params.set('workspace', args.id)
	return `${args.pathname}?${params.toString()}`
}
