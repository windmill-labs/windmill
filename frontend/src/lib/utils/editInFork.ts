import { base } from '$lib/base'
import { get } from 'svelte/store'
import {
	userStore,
	userWorkspaces,
	workspaceStore,
	type UserWorkspace,
	type UserExt
} from '$lib/stores'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
import { isRuleActive, canUserBypassRuleKind } from '$lib/workspaceProtectionRules.svelte'

type ItemType = 'script' | 'flow' | 'app' | 'raw_app'

/**
 * Whether to show the "edit in fork / dev workspace" affordance. Allowed when forking isn't disabled,
 * when the user can bypass the forking rule (workspace admins, mirroring `canCreateFork`), OR when the
 * current workspace has a canonical dev to route to — routing into an existing dev workspace creates
 * no fork, so it survives a locked prod that has `DisableWorkspaceForking` set. User identity is read
 * non-reactively (it's stable within a session); reactivity comes from the workspace args.
 */
export function editInForkAllowed(
	currentWorkspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): boolean {
	return (
		!isRuleActive('DisableWorkspaceForking') ||
		canUserBypassRuleKind('DisableWorkspaceForking', get(userStore)) ||
		!!findCanonicalDevWorkspace(currentWorkspaceId, allWorkspaces)
	)
}

/** Label for the affordance: "Edit in <dev name>" when routed to a canonical dev, else "Edit in fork". */
export function editInForkLabel(
	currentWorkspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): string {
	const dev = findCanonicalDevWorkspace(currentWorkspaceId, allWorkspaces)
	return dev ? `Edit in ${dev.name}` : 'Edit in fork'
}

/**
 * Whether the user may CREATE a new fork of the current workspace: forking not disabled, or the user
 * can bypass the rule (workspace admins). Keeps the "Fork workspace" entry available to admins as the
 * last-resort escape hatch on a locked prod.
 */
export function canCreateFork(user: UserExt | undefined): boolean {
	return (
		!isRuleActive('DisableWorkspaceForking') ||
		canUserBypassRuleKind('DisableWorkspaceForking', user)
	)
}

function editPathFor(itemType: ItemType, itemPath: string): string {
	switch (itemType) {
		case 'script':
			return `${base}/scripts/edit/${itemPath}`
		case 'flow':
			return `${base}/flows/edit/${itemPath}`
		case 'app':
			return `${base}/apps/edit/${itemPath}`
		case 'raw_app':
			return `${base}/apps_raw/edit/${itemPath}`
	}
}

function viewPathFor(itemType: ItemType, itemPath: string): string {
	switch (itemType) {
		case 'script':
			return `${base}/scripts/get/${itemPath}`
		case 'flow':
			return `${base}/flows/get/${itemPath}`
		case 'app':
			return `${base}/apps/get/${itemPath}`
		case 'raw_app':
			return `${base}/apps_raw/get/${itemPath}`
	}
}

export function buildForkEditUrl(itemType: ItemType, itemPath: string): string {
	// When the current ("prod") workspace has a canonical dev workspace, edits are funneled there:
	// land on the item's page in the dev workspace (not straight in the editor) so the workspace
	// switch is legible and the user opens the editor deliberately from there.
	const dev = findCanonicalDevWorkspace(get(workspaceStore), get(userWorkspaces))
	if (dev) {
		return `${viewPathFor(itemType, itemPath)}?workspace=${encodeURIComponent(dev.id)}`
	}
	return `${base}/user/fork_workspace?rd=${encodeURIComponent(editPathFor(itemType, itemPath))}`
}
