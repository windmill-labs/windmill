import { derived } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
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
