import { AppService, type AppWithLastVersion, type Policy } from '$lib/gen'

export type AppAccessMode = 'publisher' | 'guest' | 'anonymous'

export const CUSTOM_PATH_RE = /^[\w-]+(\/[\w-]+)*$/

/** The access a policy grants, on the three rungs the Share dialog offers. `viewer` runs the
 *  app as its viewer, which a guest cannot be, so it shows as members-only. */
export function accessModeOf(policy: Policy | undefined): AppAccessMode {
	const mode = policy?.execution_mode
	return mode === 'anonymous' ? 'anonymous' : mode === 'guest' ? 'guest' : 'publisher'
}

/** Why a custom path cannot be used, or undefined. `own` is the app's current custom path. */
export async function customPathError(
	workspace: string,
	customPath: string,
	own: string | undefined
): Promise<string | undefined> {
	if (!CUSTOM_PATH_RE.test(customPath)) return 'Use letters, digits, - and _, separated by /'
	if (customPath !== own && (await AppService.customPathExists({ workspace, customPath }))) {
		return 'Path already taken'
	}
	return undefined
}

/**
 * Change who can open a deployed app, or its custom URL, without deploying it.
 *
 * The mode is sent alone, never in a policy: the server applies it to the deployed policy under
 * the lock a deploy takes, so neither overwrites the other, and keeps the run-as user. No `value`
 * is sent, so no new version is written, and `skip_draft_deletion` keeps every draft. Resolves to
 * the row as it is after the write.
 */
export async function updateDeployedAppAccess(
	workspace: string,
	path: string,
	change: { execution_mode: AppAccessMode } | { custom_path: string | null }
): Promise<AppWithLastVersion> {
	if ('execution_mode' in change) {
		await AppService.updateApp({
			workspace,
			path,
			requestBody: { execution_mode: change.execution_mode, skip_draft_deletion: true }
		})
	} else {
		// The backend clears the custom path on an empty string and keeps it when absent.
		await AppService.updateApp({
			workspace,
			path,
			requestBody: { custom_path: change.custom_path ?? '', skip_draft_deletion: true }
		})
	}
	return AppService.getAppLiteByPath({ workspace, path })
}
