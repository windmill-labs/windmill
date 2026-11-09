import { AppService, type AppWithLastVersion, type Policy } from '$lib/gen'
import { isCloudHosted } from '$lib/cloud'

export type AppAccessMode = 'publisher' | 'guest' | 'anonymous'

export const CUSTOM_PATH_RE = /^[\w-]+(\/[\w-]+)*$/

/** What a custom path is appended to. The workspace is in it on the shared cloud and wherever the
 *  instance setting `app_workspaced_route` is on. */
export function customUrlBaseOf(workspace: string, workspacedRoute: boolean): string {
	return `${window.location.origin}/a/${isCloudHosted() || workspacedRoute ? workspace + '/' : ''}`
}

export function secretUrlOf(workspace: string, secret: string): string {
	return `${window.location.origin}/public/${workspace}/${secret}`
}

/** The access a policy grants, on the three rungs the Share dialog offers. `viewer` runs the
 *  app as its viewer, which a guest cannot be, so it shows as members-only. */
export function accessModeOf(policy: Policy | undefined): AppAccessMode {
	const mode = policy?.execution_mode
	return mode === 'anonymous' ? 'anonymous' : mode === 'guest' ? 'guest' : 'publisher'
}

/**
 * Why the server refuses to change who can open a deployed app, or undefined. Mirrors the
 * mode-only checks of `update_app_internal`: an app that runs as someone else is refused to a
 * caller who may not keep that run-as user and to a compose-only builder, who is also refused on
 * an app that is not sandboxed. A deploy by the caller lifts either refusal, except that a builder
 * who may keep the run-as user (`builder_run_as`) keeps it unless they pick themselves first.
 */
export function modeChangeRefusal(
	policy: Pick<Policy, 'on_behalf_of' | 'sandbox'>,
	opts: { username: string | undefined; canPreserveRunAs: boolean; composeOnly: boolean }
): 'run_as' | 'builder_run_as' | 'unsandboxed' | undefined {
	const runsAsSomeoneElse = !!policy.on_behalf_of && policy.on_behalf_of !== `u/${opts.username}`
	if (runsAsSomeoneElse && opts.composeOnly && opts.canPreserveRunAs) return 'builder_run_as'
	if (runsAsSomeoneElse && (opts.composeOnly || !opts.canPreserveRunAs)) return 'run_as'
	if (opts.composeOnly && policy.sandbox !== true) return 'unsandboxed'
	return undefined
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
