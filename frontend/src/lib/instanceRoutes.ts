/**
 * Pages that read only instance-level endpoints, so a devops user can open them without
 * belonging to any workspace. Devops grants no workspace access, so a page added here must not
 * call a workspace-scoped endpoint or read `$workspaceStore`/`$userStore` to render.
 */
const INSTANCE_ROUTES = ['/workers', '/service_logs']

export function isInstanceRoute(pathname: string): boolean {
	return INSTANCE_ROUTES.some((r) => pathname === r || pathname.startsWith(r + '/'))
}
