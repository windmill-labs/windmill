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
	},
	{
		value: 'flow_conversations:read',
		label: 'Read your flow chats',
		description: 'List your chat conversations with flows and read their messages'
	},
	{
		value: 'flow_conversations:write',
		label: 'Manage your flow chats',
		description: 'Read and delete your chat conversations with flows'
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
// `preview` keeps the editor's consent apart from the deployed app's: the editor
// runs a draft nobody may have reviewed, so trust given to one is not trust given
// to the other.
function sdkConsentKey(viewer: string, workspace: string, path: string, preview: boolean): string {
	return `wm_sdk_consent:${preview ? 'p:' : ''}${viewer}:${workspace}:${path}`
}

/** True when `approved` (a consent the viewer gave) covers every scope the app now
 * declares. Fewer scopes need no new consent; one more does. */
export function sdkConsentCovers(approved: unknown, scopes: string[]): boolean {
	return Array.isArray(approved) && scopes.every((s) => approved.includes(s))
}

/** True when a previously stored "do not ask again" consent covers every
 * declared scope. A later deploy that adds scopes re-triggers the prompt. */
export function hasStoredSdkConsent(
	viewer: string,
	workspace: string,
	path: string,
	scopes: string[],
	preview = false
): boolean {
	try {
		const stored = JSON.parse(
			localStorage.getItem(sdkConsentKey(viewer, workspace, path, preview)) ?? 'null'
		)
		return sdkConsentCovers(stored, scopes)
	} catch (_) {
		return false
	}
}

export function storeSdkConsent(
	viewer: string,
	workspace: string,
	path: string,
	scopes: string[],
	preview = false
): void {
	try {
		localStorage.setItem(sdkConsentKey(viewer, workspace, path, preview), JSON.stringify(scopes))
	} catch (_) {}
}

/** Follows the editor's app to its new path (first deploy of a draft, or a rename),
 * so a stored "do not ask again" consent neither is asked again nor stays behind
 * under the old path. A plain Continue is not stored and does not carry over. */
export function movePreviewSdkConsent(
	viewer: string,
	workspace: string,
	from: string,
	to: string
): void {
	if (from === to) return
	try {
		const fromKey = sdkConsentKey(viewer, workspace, from, true)
		const stored = localStorage.getItem(fromKey)
		if (stored === null) return
		localStorage.setItem(sdkConsentKey(viewer, workspace, to, true), stored)
		localStorage.removeItem(fromKey)
	} catch (_) {}
}
