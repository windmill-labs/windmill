import { UserService } from '$lib/gen'

export const DEFAULT_SUPERADMIN_EMAIL = 'admin@windmill.dev'

/** The quick setup's step that replaces the default superadmin account. That step deletes
 * the signed-in account, so only send the default account itself there. */
export const REPLACE_DEFAULT_SUPERADMIN_HREF = '/user/instance_settings?step=account'

/** Whether admin@windmill.dev can still sign in with changeme. Superadmin-only endpoint. */
export async function isDefaultSuperadminActive(): Promise<boolean> {
	try {
		return await UserService.isDefaultSuperadminActive()
	} catch (e) {
		console.error('Could not check the default superadmin account', e)
		return false
	}
}

/** Whether the signed-in user is the default account and it still has its default password. */
export async function isSignedInAsActiveDefaultSuperadmin(): Promise<boolean> {
	try {
		if ((await UserService.getCurrentEmail()) !== DEFAULT_SUPERADMIN_EMAIL) return false
	} catch {
		return false
	}
	return isDefaultSuperadminActive()
}
