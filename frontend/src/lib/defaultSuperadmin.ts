import { UserService } from '$lib/gen'

/** The quick setup's step that replaces the default superadmin account. */
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
