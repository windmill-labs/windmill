/**
 * Pages a devops user can open without belonging to any workspace. Devops grants no workspace
 * access, so such a page renders with `$workspaceStore` and `$userStore` unset and must hide
 * whatever needs them (a workspace-scoped call or a link into a workspace) rather than break.
 */
const INSTANCE_ROUTES = ['/workers', '/service_logs']

export function isInstanceRoute(pathname: string): boolean {
	return INSTANCE_ROUTES.some((r) => pathname === r || pathname.startsWith(r + '/'))
}
