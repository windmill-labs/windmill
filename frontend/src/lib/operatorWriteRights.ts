import { derived, fromStore, get, readable, type Readable } from 'svelte/store'
import type { RoleCapabilities } from '$lib/gen'
import { userStore, userWorkspaces, usersWorkspaceStore, workspaceStore } from '$lib/stores'
import {
	useOperatingWorkspace,
	useReadOnlyWorkspace
} from '$lib/components/operatingWorkspace.svelte'
import { useActingUser } from '$lib/actingUser.svelte'
import { canDraftItem, canEditItem, roleCanAuthor, roleCanDraft } from '$lib/editRights'
import { capabilitiesIn, withoutBuilding } from '$lib/roleCapabilities'

/**
 * The `editRights` rules (`roleCanAuthor` / `canEditItem` for the item handlers, `roleCanDraft` /
 * `canDraftItem` for drafts) answered for the user acting in the operating workspace, with their
 * role's capabilities there, or in `workspace`, for a component that is handed one rather than
 * sitting under a host that declares it. Reads context and registers an effect: call during
 * component initialisation.
 */
export function useEditRights(workspace?: () => string | undefined) {
	const operating = fromStore(useOperatingWorkspace())
	const ws = workspace ?? (() => operating.current)
	const user = useActingUser(ws)
	const readOnly = useReadOnlyWorkspace()
	const workspaces = fromStore(userWorkspaces)
	const open = () => !readOnly(ws())
	// A read-only workspace leaves the role building nothing, so a gate reading `capabilities`
	// follows it as the four checks below do. Those keep their own guard: every role edits
	// resources and variables.
	const caps = () => {
		const roleCaps = capabilitiesIn(
			user.current,
			workspaces.current.find((w) => w.id === ws())
		)
		return open() ? roleCaps : withoutBuilding(roleCaps)
	}
	return {
		get capabilities(): RoleCapabilities {
			return caps()
		},
		roleCanAuthor: (kind: string) => open() && roleCanAuthor(kind, caps()),
		canEditItem: (kind: string, path: string, extraPerms: Record<string, boolean> | undefined) =>
			open() && canEditItem(kind, path, extraPerms, user.current, caps()),
		roleCanDraft: (kind: string) => open() && roleCanDraft(kind, caps()),
		canDraftItem: (kind: string, path: string, extraPerms: Record<string, boolean> | undefined) =>
			open() && canDraftItem(kind, path, extraPerms, user.current, caps()),
		/** This hook looks the user up on its own, so an editor that retries a failed lookup on
		 *  its acting user (`forgetFailures`) must retry this one too, or it stays read-only. */
		forgetFailures: () => user.forgetFailures()
	}
}

/**
 * The role's capabilities in `workspace`, from the workspace list alone: `$userStore` describes
 * only the navigation workspace, so elsewhere only superadmin carries over. Admins get full
 * capabilities from the server's list.
 */
function capabilitiesStore(workspace: Readable<string | undefined>): Readable<RoleCapabilities> {
	return derived(
		[userStore, workspaceStore, userWorkspaces, workspace],
		([$user, $nav, $workspaces, $ws]) =>
			capabilitiesIn(
				$ws === $nav && $user?.workspace_id === $ws
					? $user
					: { is_super_admin: $user?.is_super_admin },
				$workspaces.find((w) => w.id === $ws)
			)
	)
}

/** The role's capabilities in the operating workspace. Reads context: call during component
 * initialisation. Code outside any component reads `roleCapabilities` (navigation workspace) or
 * `capabilitiesInWorkspace`. */
export const useRoleCapabilities = () => capabilitiesStore(useOperatingWorkspace())

/** The role's capabilities in a given workspace, read once: for code acting on an explicit
 * workspace rather than the nav's, such as the global AI chat. */
export const capabilitiesInWorkspace = (workspace: string | undefined) =>
	get(capabilitiesStore(readable(workspace)))

/**
 * Why writes of this kind are locked in the operating workspace, or `undefined` when they are not:
 * the shape `title` and `disabled` both want. See `docs/operator-write-rights.md`.
 */
function writeLock(kind: 'schedule' | 'trigger', noun: string) {
	return derived(useRoleCapabilities(), (caps) =>
		// Worded as the server words its refusal of the same write.
		roleCanAuthor(kind, caps) ? undefined : `Your role cannot manage ${noun} in this workspace`
	)
}

/** Reads context: call during component initialisation. */
export const useScheduleLock = () => writeLock('schedule', 'schedules')
/** Reads context: call during component initialisation. */
export const useTriggerLock = () => writeLock('trigger', 'triggers')

/** Resolves once the workspace list has loaded. Capabilities fall back to the user's legacy
 * operator flag until then, so code that picks what a builder starts with on a fresh page load
 * has to wait for it. */
export async function workspaceListLoaded(): Promise<void> {
	let unsubscribe = () => {}
	await new Promise<void>((resolve) => {
		unsubscribe = usersWorkspaceStore.subscribe((list) => list && resolve())
	})
	unsubscribe()
}
