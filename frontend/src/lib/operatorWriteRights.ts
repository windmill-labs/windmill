import { derived, type Readable } from 'svelte/store'
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
