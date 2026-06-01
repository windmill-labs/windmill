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
 * Whether the `?workspace=` query param should be kept in sync with the active
 * workspace store on the given route. Drives both directions of the sync in the
 * logged layout: URL → store on navigation, and store → URL via replaceState.
 */
export function workspaceParamAllowed(pathname: string): boolean {
	return !WORKSPACE_PARAM_EXCLUDED_PREFIXES.some((prefix) => pathname.startsWith(prefix))
}
