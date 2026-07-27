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
import { goto } from '$lib/navigation'
import { checkItemExists } from '$lib/utils_workspace_deploy'
import { pullIntoDevModal } from '$lib/utils/editInForkModal.svelte'

export type ItemType = 'script' | 'flow' | 'app' | 'raw_app'

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

/** Item page in the dev workspace — the target `buildForkEditUrl` produces when a dev exists. */
export function devWorkspaceItemUrl(
	itemType: ItemType,
	itemPath: string,
	devWorkspaceId: string
): string {
	return `${viewPathFor(itemType, itemPath)}?workspace=${encodeURIComponent(devWorkspaceId)}`
}

/**
 * A dev workspace can be behind its prod, so the URL built at render time dead-ends on a not-found
 * page for any item prod has and dev doesn't. Resolve the destination at click time instead:
 * return it when the item is there, else raise the prompt offering to copy it over and return
 * undefined. Shared by the row buttons and the editors' "Edit in <dev>" dropdown entries.
 */
async function resolveEditInForkTarget(
	itemType: ItemType,
	itemPath: string,
	prod: string,
	dev: UserWorkspace
): Promise<string | undefined> {
	let exists: boolean
	try {
		exists = await checkItemExists(itemType, itemPath, dev.id)
	} catch {
		// Inconclusive — go anyway and let the item page report whatever is actually wrong.
		exists = true
	}
	if (exists) return devWorkspaceItemUrl(itemType, itemPath, dev.id)
	pullIntoDevModal.val = {
		itemType,
		itemPath,
		devWorkspaceId: dev.id,
		devWorkspaceName: dev.name,
		prodWorkspaceId: prod
	}
	return undefined
}

function currentDevWorkspace(): { prod: string; dev: UserWorkspace } | undefined {
	const prod = get(workspaceStore)
	const dev = findCanonicalDevWorkspace(prod, get(userWorkspaces))
	if (!dev || !prod) return undefined
	return { prod, dev }
}

/**
 * Click handler for the "Edit in <dev workspace>" affordance on a link. Modifier/middle clicks fall
 * through to the raw href so open-in-new-tab keeps working.
 */
export async function onEditInForkClick(
	e: Event | undefined,
	itemType: ItemType,
	itemPath: string
): Promise<void> {
	const click = e as MouseEvent | undefined
	if (click?.ctrlKey || click?.metaKey || click?.shiftKey || click?.altKey || click?.button) return
	// No dev workspace: the href is the fork-creation flow, which needs no resolving.
	const target = currentDevWorkspace()
	if (!target) return
	e?.preventDefault()
	const url = await resolveEditInForkTarget(itemType, itemPath, target.prod, target.dev)
	if (url) await goto(url)
}

/**
 * "Edit in <dev workspace>" from an editor's dropdown, which opens a new tab rather than navigating
 * away from work in progress. Popups opened after the existence check can be blocked once the
 * click's transient activation has lapsed — navigate in place then, rather than doing nothing.
 */
export async function openEditInFork(itemType: ItemType, itemPath: string): Promise<void> {
	const target = currentDevWorkspace()
	if (!target) {
		window.open(buildForkEditUrl(itemType, itemPath))
		return
	}
	const url = await resolveEditInForkTarget(itemType, itemPath, target.prod, target.dev)
	if (!url) return
	if (!window.open(url)) await goto(url)
}
