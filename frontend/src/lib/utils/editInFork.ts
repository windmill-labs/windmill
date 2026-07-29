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
import { sendUserToast } from '$lib/toast'
import { checkItemExists } from '$lib/utils_workspace_deploy'
import { updateDevWorkspaceModal } from '$lib/utils/editInForkModal.svelte'

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

export function buildForkEditUrl(itemType: ItemType, itemPath: string): string {
	// When the current ("prod") workspace has a canonical dev workspace, edits are funneled there.
	const dev = findCanonicalDevWorkspace(get(workspaceStore), get(userWorkspaces))
	return dev
		? devWorkspaceEditUrl(itemType, itemPath, dev.id)
		: forkWorkspaceUrl(itemType, itemPath)
}

/** Fork-creation flow, coming back to the item's editor once the fork exists. */
export function forkWorkspaceUrl(itemType: ItemType, itemPath: string): string {
	return `${base}/user/fork_workspace?rd=${encodeURIComponent(editPathFor(itemType, itemPath))}`
}

/** The item's editor in the dev workspace — the target `buildForkEditUrl` produces when a dev exists. */
export function devWorkspaceEditUrl(
	itemType: ItemType,
	itemPath: string,
	devWorkspaceId: string
): string {
	// `?workspace=` switches the workspace store (handled in the logged layout), so the editor
	// opens against the dev workspace rather than whichever one the tab was on.
	return `${editPathFor(itemType, itemPath)}?workspace=${encodeURIComponent(devWorkspaceId)}`
}

/**
 * A dev workspace can be behind its prod, so the URL built at render time dead-ends on a not-found
 * page for any item prod has and dev doesn't. Resolve the destination at click time instead:
 * return it when the item is there, else raise the prompt offering to update the dev workspace with
 * it and return undefined. Shared by the row buttons and the editors' "Edit in <dev>" dropdown
 * entries.
 */
let latestResolve = 0

async function resolveEditInForkTarget(
	itemType: ItemType,
	itemPath: string,
	prod: string,
	dev: UserWorkspace
): Promise<string | undefined> {
	const seq = ++latestResolve
	const from = { path: window.location.pathname, workspace: get(workspaceStore) }
	let exists: boolean
	try {
		exists = await checkItemExists(itemType, itemPath, dev.id)
	} catch {
		// Inconclusive — go anyway and let the editor report whatever is actually wrong.
		exists = true
	}
	// Only act if the user is still where they asked from. A later click supersedes this one, and
	// navigating or switching workspace abandons it — the modal is layout-global and `goto` is
	// unconditional, so a late answer would otherwise hijack whatever they moved on to.
	if (seq !== latestResolve) return undefined
	if (window.location.pathname !== from.path || get(workspaceStore) !== from.workspace)
		return undefined
	if (exists) return devWorkspaceEditUrl(itemType, itemPath, dev.id)
	updateDevWorkspaceModal.val = {
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
 * Click handler for the "Edit in <dev workspace>" affordance. Menu entries carry no href — the
 * destination is only known after an async probe — so this navigates itself by default. Link
 * callers pass `hasHref` so modifier/middle clicks still open the raw href in a new tab, and so the
 * no-dev-workspace case is left to the anchor rather than being navigated twice.
 */
export async function onEditInForkClick(
	e: Event | undefined,
	itemType: ItemType,
	itemPath: string,
	{ hasHref = false }: { hasHref?: boolean } = {}
): Promise<void> {
	const click = e as MouseEvent | undefined
	if (
		hasHref &&
		(click?.ctrlKey || click?.metaKey || click?.shiftKey || click?.altKey || click?.button)
	)
		return
	const target = currentDevWorkspace()
	if (!target) {
		// Nothing to probe: the destination is the fork-creation flow, which the anchor already points at.
		if (!hasHref) await goto(forkWorkspaceUrl(itemType, itemPath))
		return
	}
	e?.preventDefault()
	const url = await resolveEditInForkTarget(itemType, itemPath, target.prod, target.dev)
	if (url) await goto(url)
}

/**
 * "Edit in <dev workspace>" from an editor's dropdown, which opens a new tab rather than navigating
 * away from work in progress.
 */
export async function openEditInFork(itemType: ItemType, itemPath: string): Promise<void> {
	const target = currentDevWorkspace()
	if (!target) {
		window.open(buildForkEditUrl(itemType, itemPath))
		return
	}
	const url = await resolveEditInForkTarget(itemType, itemPath, target.prod, target.dev)
	if (!url) return
	if (!window.open(url)) {
		// Blocked, because the existence check outlived the click's transient activation. Falling
		// back to navigating in place would throw away whatever this editor is holding — the whole
		// reason this entry opens a tab — so say so instead.
		sendUserToast(`Allow popups to open ${itemPath} in ${target.dev.name}`, true)
	}
}
