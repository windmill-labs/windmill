// Routes that must never carry the `?workspace=` query param.
//
// `/user/*` covers auth, workspace selection, onboarding, invites and
// instance-level settings — none of which are scoped to an active workspace
// (and several run before one is chosen). `/oauth/*` covers OAuth callbacks
// and `mcp_authorize`, which carries its own `workspace_id`.
//
// Public, path-scoped routes (`/a`, `/public`, `/approve`,
// `/apps_raw/[workspace]`) live outside the logged layout, so this predicate is
// never consulted for them — the workspace already lives in their path.
const WORKSPACE_PARAM_EXCLUDED_PREFIXES = ['/user/', '/oauth/']

/**
 * Whether the `?workspace=` query param may appear at all on the given route.
 * Hard exclusion: drives both directions of the sync in the logged layout — when
 * false the param is never read into the store nor written to the URL.
 */
export function workspaceParamAllowed(pathname: string): boolean {
	return !WORKSPACE_PARAM_EXCLUDED_PREFIXES.some((prefix) => pathname.startsWith(prefix))
}

// Routes whose content does not depend on the active workspace (instance-level
// pages). The param is meaningless here, so the URL is kept clean: an explicit
// `?workspace=` is still adopted into the store — so a shared link propagates the
// workspace to subsequent workspace-scoped pages — and then stripped. The active
// workspace is preserved by the store + storage, not by the URL on these routes.
const WORKSPACE_AGNOSTIC_PREFIXES = [
	'/workers',
	'/service_logs',
	'/instance_groups',
	'/concurrency_groups'
]

/**
 * Whether the given route is workspace-agnostic (instance-level). On these
 * routes the logged layout strips `?workspace=` rather than seeding it.
 */
export function workspaceAgnosticRoute(pathname: string): boolean {
	return WORKSPACE_AGNOSTIC_PREFIXES.some(
		(prefix) => pathname === prefix || pathname.startsWith(prefix + '/')
	)
}
