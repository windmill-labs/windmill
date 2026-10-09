import { derived, fromStore, get, readable, type Readable } from 'svelte/store'
import { userWorkspaces, usersWorkspaceStore, workspaceStore } from '$lib/stores'
import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
import { useActingUser } from '$lib/actingUser.svelte'
import { canDraftItem, canEditItem, roleCanAuthor, roleCanDraft } from '$lib/editRights'

/**
 * The `editRights` rules (`roleCanAuthor` / `canEditItem` for the item handlers, `roleCanDraft` /
 * `canDraftItem` for drafts) answered for the user acting in the operating workspace,
 * with that workspace's `operator_settings` — or in `workspace`, for a component that is handed
 * one rather than sitting under a host that declares it. Reads context and registers an effect:
 * call during component initialisation.
 */
export function useEditRights(workspace?: () => string | undefined) {
	const operating = fromStore(useOperatingWorkspace())
	const ws = workspace ?? (() => operating.current)
	const user = useActingUser(ws)
	const workspaces = fromStore(userWorkspaces)
	const settings = () => workspaces.current.find((w) => w.id === ws())?.operator_settings
	return {
		roleCanAuthor: (kind: string) => roleCanAuthor(kind, user.current, settings()),
		canEditItem: (kind: string, path: string, extraPerms: Record<string, boolean> | undefined) =>
			canEditItem(kind, path, extraPerms, user.current, settings()),
		roleCanDraft: (kind: string) => roleCanDraft(kind, user.current, settings()),
		canDraftItem: (kind: string, path: string, extraPerms: Record<string, boolean> | undefined) =>
			canDraftItem(kind, path, extraPerms, user.current, settings()),
		/** This hook looks the user up on its own, so an editor that retries a failed lookup on
		 *  its acting user (`forgetFailures`) must retry this one too, or it stays read-only. */
		forgetFailures: () => user.forgetFailures()
	}
}

/**
 * Why writes of this kind are locked in the operating workspace, or `undefined` when they are not —
 * the shape `title` and `disabled` both want. See `docs/operator-write-rights.md`.
 *
 * The role half of `roleCanAuthor`, phrased for the controls that explain it. `operator_settings`
 * is non-null exactly for an operator (`isOperatorInWorkspace`), so it stands in for the user.
 */
function writeLock(kind: 'schedule' | 'trigger', noun: string) {
	return derived([userWorkspaces, useOperatingWorkspace()], ([$userWorkspaces, $workspace]) => {
		const settings = $userWorkspaces.find((w) => w.id === $workspace)?.operator_settings
		// Worded as the server words its refusal of the same write.
		return roleCanAuthor(kind, { operator: settings != null }, settings)
			? undefined
			: `Operators cannot manage ${noun} in this workspace`
	})
}

/** Reads context: call during component initialisation. */
export const useScheduleLock = () => writeLock('schedule', 'schedules')
/** Reads context: call during component initialisation. */
export const useTriggerLock = () => writeLock('trigger', 'triggers')

/**
 * True when the user is an operator of `workspace` and it granted operators the right to compose
 * this kind out of runnables that are already deployed. They still author no code, and everywhere
 * else `operator` keeps meaning read-only, so a gate on the operator role has to consult this before
 * refusing. `operator_settings` is null for a non-operator, so this is false for them.
 *
 * The two rights are granted independently: anything that authors one kind must gate on that
 * kind, or a flows-only workspace offers app affordances the backend then refuses.
 */
function builderRight(
	key: 'builder_flows' | 'builder_apps',
	workspace: Readable<string | undefined>
): Readable<boolean> {
	return derived(
		[userWorkspaces, workspace],
		([$userWorkspaces, $workspace]) =>
			$userWorkspaces.find((w) => w.id === $workspace)?.operator_settings?.[key] === true
	)
}

/** Either right, only for surfaces that are not per-kind, such as the create menu's visibility. */
function anyBuilderRight(workspace: Readable<string | undefined>): Readable<boolean> {
	return derived(
		[builderRight('builder_flows', workspace), builderRight('builder_apps', workspace)],
		([flows, apps]) => flows || apps
	)
}

/** Builder rights in the operating workspace. Reads context: call during component
 * initialisation. */
export const useOperatorBuilderFlows = () => builderRight('builder_flows', useOperatingWorkspace())
/** Reads context: call during component initialisation. */
export const useOperatorBuilderApps = () => builderRight('builder_apps', useOperatingWorkspace())
/** Reads context: call during component initialisation. */
export const useOperatorBuilderRights = () => anyBuilderRight(useOperatingWorkspace())

/** Builder rights in the navigation workspace, for code outside any component: the legacy AI
 * chat is one instance for the whole app and acts on the workspace the nav is on. */
export const navigationOperatorBuilderFlows = builderRight('builder_flows', workspaceStore)
export const navigationOperatorBuilderApps = builderRight('builder_apps', workspaceStore)
export const navigationOperatorBuilderRights = anyBuilderRight(workspaceStore)

/** The app builder right in a given workspace, read once: for code acting on an explicit
 * workspace rather than the nav's, such as the global AI chat. */
export const operatorBuilderAppsIn = (workspace: string | undefined) =>
	get(builderRight('builder_apps', readable(workspace)))

/** Resolves once the workspace list has loaded. Every builder right reads false until then, so
 * code that picks what a builder starts with on a fresh page load has to wait for it. */
export async function workspaceListLoaded(): Promise<void> {
	let unsubscribe = () => {}
	await new Promise<void>((resolve) => {
		unsubscribe = usersWorkspaceStore.subscribe((list) => list && resolve())
	})
	unsubscribe()
}
