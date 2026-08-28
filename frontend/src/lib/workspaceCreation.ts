/**
 * The instance policy around creating a workspace, and what the client has to do
 * once one exists. Shared by every screen that creates workspaces — the workspace
 * settings creator and the hub import wizard — so the two cannot drift apart on
 * who is allowed to create, what username the backend expects, or what state the
 * client is left in afterwards.
 *
 * Id validation lives next door in `$lib/utils/workspaceId`, which has no imports
 * and so can be used from anywhere.
 */

import { SettingService, UserService, WorkspaceService } from '$lib/gen'
import { usersWorkspaceStore } from '$lib/stores'
import { switchWorkspace } from '$lib/storeUtils'
import { isCloudHosted } from '$lib/cloud'
import { base } from '$lib/base'

/**
 * Whether this user may create a workspace at all. Self-hosted instances default
 * `CREATE_WORKSPACE_REQUIRE_SUPERADMIN` to true, so offering the choice to
 * everyone ends in a 403 at the last step.
 *
 * Superadmin arrives asynchronously in most callers, so pass the current value and
 * call again when it flips. When the gate is on, the server is asked directly
 * rather than trusting that value: `refreshSuperadmin` skips its fetch once the
 * store holds anything, so a page loaded logged out leaves it `false` for the rest
 * of the session — including right after signing in.
 */
export async function canCreateWorkspace(isSuperadmin: boolean): Promise<boolean> {
	if (isSuperadmin || isCloudHosted()) return true
	try {
		const r = await fetch(base + '/api/workspaces/create_workspace_require_superadmin')
		if ((await r.text()) != 'true') return true
		return !!(await UserService.globalWhoami()).super_admin
	} catch {
		return false
	}
}

export interface UsernamePolicy {
	/** When true the backend derives the username and rejects one sent explicitly. */
	automate: boolean
	/** A username to prefill with, when the caller has to ask for one. */
	suggested?: string
}

/**
 * `createWorkspace` rejects a username when the instance automates them and
 * requires one when it does not, so the field only exists in the second case.
 */
export async function loadUsernamePolicy(): Promise<UsernamePolicy> {
	const automate =
		((await SettingService.getGlobal({
			key: 'automate_username_creation'
		})) as boolean | null) ?? true
	if (automate) return { automate: true }
	try {
		const me = await UserService.globalWhoami()
		const from = me.name ? me.name.split(' ')[0] : me.email.split('@')[0]
		return { automate: false, suggested: from.replace(/\./g, '').toLowerCase() }
	} catch {
		return { automate: false }
	}
}

/**
 * Re-reads the workspaces this user belongs to. Anything that creates or deletes a
 * workspace owes the client this call: `usersWorkspaceStore` is what the picker and
 * every derived workspace list read from, and nothing else refreshes it until a
 * full page load.
 */
export async function refreshWorkspaceList(): Promise<void> {
	usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
}

/** Enters a workspace that was just created, with the list refreshed to match. */
export async function enterNewWorkspace(id: string): Promise<void> {
	await refreshWorkspaceList()
	switchWorkspace(id)
}
