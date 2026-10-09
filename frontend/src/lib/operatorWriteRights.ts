import { derived, fromStore, type Readable } from 'svelte/store'
import { userWorkspaces, workspaceStore } from '$lib/stores'
import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
import { useActingUser } from '$lib/actingUser.svelte'
import { canEditItem, roleCanAuthor } from '$lib/editRights'

/**
 * `roleCanAuthor` and `canEditItem` answered for the user acting in the operating workspace,
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
 * flows out of runnables that are already deployed. They still author no code, and everywhere else
 * `operator` keeps meaning read-only, so a gate on the operator role has to consult this before
 * refusing. `operator_settings` is null for a non-operator, so this is false for them.
 */
function builderFlows(workspace: Readable<string | undefined>): Readable<boolean> {
	return derived(
		[userWorkspaces, workspace],
		([$userWorkspaces, $workspace]) =>
			$userWorkspaces.find((w) => w.id === $workspace)?.operator_settings?.builder_flows === true
	)
}

/** Builder rights in the operating workspace. Reads context: call during component
 * initialisation. */
export const useOperatorBuilderFlows = () => builderFlows(useOperatingWorkspace())

/** Builder rights in the navigation workspace, for code outside any component: the legacy AI
 * chat is one instance for the whole app and acts on the workspace the nav is on. */
export const navigationOperatorBuilderFlows = builderFlows(workspaceStore)
