// Href for a workspace-switch link in the sidebar WorkspaceMenu.
//
// On an AI-session route, switching workspace leaves for home — but we keep the
// `?workspace=<id>` param so a modifier/middle click (open in new tab, which
// bypasses the onClick fast-path) still lands in the *clicked* workspace's home
// rather than the default one. Everywhere else, stay on the current path and
// just swap the `workspace` query param.
export function workspaceMenuHref(args: {
	routeId: string | null | undefined
	base: string
	pathname: string
	searchParams: URLSearchParams
	id: string
}): string {
	if (args.routeId?.includes('/sessions')) {
		return `${args.base}/?workspace=${args.id}`
	}
	const params = new URLSearchParams(args.searchParams)
	params.set('workspace', args.id)
	return `${args.pathname}?${params.toString()}`
}
