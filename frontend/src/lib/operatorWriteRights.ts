import { derived, get, readable, type Readable } from 'svelte/store'
import { userWorkspaces, workspaceStore } from '$lib/stores'
import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

/**
 * Why writes of this kind are locked in the operating workspace, or `undefined` when they are not —
 * the shape `title` and `disabled` both want. See `docs/operator-write-rights.md`.
 *
 * Only `false` locks. A workspace that never configured the key and a non-operator (whose
 * `operator_settings` is null) both hold the right, so `=== true` here would disable the controls
 * for everyone.
 */
function writeLock(key: 'manage_schedules' | 'manage_triggers', noun: string) {
	return derived([userWorkspaces, useOperatingWorkspace()], ([$userWorkspaces, $workspace]) => {
		const settings = $userWorkspaces.find((w) => w.id === $workspace)?.operator_settings
		// Worded as the server words its refusal of the same write.
		return settings?.[key] === false
			? `Operators cannot manage ${noun} in this workspace`
			: undefined
	})
}

/** Reads context: call during component initialisation. */
export const useScheduleLock = () => writeLock('manage_schedules', 'schedules')
/** Reads context: call during component initialisation. */
export const useTriggerLock = () => writeLock('manage_triggers', 'triggers')

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
