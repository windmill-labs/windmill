import { BROWSER } from 'esm-env'

export function isCloudHosted(): boolean {
	return BROWSER && window.location.hostname == 'app.windmill.dev'
}

// On the managed cloud, the public demo workspace is kept clean and consistent by
// disabling folder creation, item sharing, and group creation for non-admins. The
// backend enforces the same rule; this only drives the UI hints.
export const DEMO_RESTRICTION_HINT =
	'Disabled in the demo workspace. Create your own workspace to keep the demo clean and consistent.'

export function isDemoWorkspaceRestricted(
	workspace: string | undefined,
	isAdmin: boolean | undefined,
	isSuperAdmin: boolean | undefined
): boolean {
	return isCloudHosted() && workspace === 'demo' && !isAdmin && !isSuperAdmin
}
