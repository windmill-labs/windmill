// Frontend-SDK permissions for raw apps: the curated scopes an app author may
// declare in `policy.frontend_sdk_scopes` (mirrors FRONTEND_SDK_ALLOWED_SCOPES
// in the backend `apps.rs` — both lists must stay in sync), plus the viewer-side
// consent persistence for the permission banner.

export const FRONTEND_SDK_SCOPES: { value: string; label: string; description: string }[] = [
	{
		value: 'jobs:run',
		label: 'Run scripts and flows',
		description: 'Execute any script or flow the viewer can run, and read jobs'
	},
	{
		value: 'jobs:read',
		label: 'Read jobs and results',
		description: 'Read jobs and their results, and list the runs the viewer can see'
	},
	{
		value: 'users:read',
		label: 'Read your identity',
		description: 'Call whoami (Read the viewer username, email, groups, is_super_admin...)'
	},
	{
		value: 'resources:read',
		label: 'Read resources',
		description: 'Read resource values the viewer can access, including credentials'
	},
	{
		value: 'variables:read',
		label: 'Read variables',
		description: 'Read variable values the viewer can access'
	}
]

export function sdkScopeLabel(scope: string): string {
	return FRONTEND_SDK_SCOPES.find((s) => s.value === scope)?.label ?? scope
}

export function sdkScopeDescription(scope: string): string | undefined {
	return FRONTEND_SDK_SCOPES.find((s) => s.value === scope)?.description
}

// Keyed by viewer as well as app: this localStorage lives on the shared embedder
// origin, so without the viewer one person's "do not ask again" would silently
// suppress the prompt for the next person to use the same browser profile.
function sdkConsentKey(viewer: string, workspace: string, path: string): string {
	return `wm_sdk_consent:${viewer}:${workspace}:${path}`
}

/** True when a previously stored "do not ask again" consent covers every
 * declared scope. A later deploy that adds scopes re-triggers the prompt. */
export function hasStoredSdkConsent(
	viewer: string,
	workspace: string,
	path: string,
	scopes: string[]
): boolean {
	try {
		const stored = JSON.parse(
			localStorage.getItem(sdkConsentKey(viewer, workspace, path)) ?? 'null'
		)
		return Array.isArray(stored) && scopes.every((s) => stored.includes(s))
	} catch (_) {
		return false
	}
}

export function storeSdkConsent(
	viewer: string,
	workspace: string,
	path: string,
	scopes: string[]
): void {
	try {
		localStorage.setItem(sdkConsentKey(viewer, workspace, path), JSON.stringify(scopes))
	} catch (_) { }
}
