import { derived } from 'svelte/store'
import { userWorkspaces, workspaceStore } from '$lib/stores'

/**
 * Why writes of this kind are locked in the active workspace, or `undefined` when they are not —
 * the shape `title` and `disabled` both want. See `docs/operator-write-rights.md`.
 *
 * Only `false` locks. A workspace that never configured the key and a non-operator (whose
 * `operator_settings` is null) both hold the right, so `=== true` here would disable the controls
 * for everyone.
 */
function writeLock(key: 'manage_schedules' | 'manage_triggers', noun: string) {
	return derived([userWorkspaces, workspaceStore], ([$userWorkspaces, $workspaceStore]) => {
		const settings = $userWorkspaces.find((w) => w.id === $workspaceStore)?.operator_settings
		// Worded as the server words its refusal of the same write.
		return settings?.[key] === false
			? `Operators cannot manage ${noun} in this workspace`
			: undefined
	})
}

export const scheduleLock = writeLock('manage_schedules', 'schedules')
export const triggerLock = writeLock('manage_triggers', 'triggers')
