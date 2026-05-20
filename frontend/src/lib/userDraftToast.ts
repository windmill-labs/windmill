/**
 * "Restored from local storage" toast, shown when an editor reopens on a
 * local autosave that differs from the backend. Owns only the wording and
 * which reset actions are offered; the reset side-effects live at each call
 * site (route-specific state).
 */
import { sendUserToast } from '$lib/toast'

export type RestoreFromLocalActions = {
	/** Drop the local autosave, apply the backend DB draft. Offered when `hasBackendDraft`. */
	onResetToSavedDraft?: () => void | Promise<void>
	/** Drop the local autosave, load the deployed version. Offered when `hasDeployed`. */
	onResetToDeployed?: () => void | Promise<void>
}

/** Show the toast with up to two reset actions, gated by what the backend has. */
export function notifyRestoredFromLocal(
	hasBackendDraft: boolean,
	hasDeployed: boolean,
	{ onResetToSavedDraft, onResetToDeployed }: RestoreFromLocalActions
): void {
	const actions: Array<{ label: string; callback: () => void | Promise<void> }> = []
	if (hasBackendDraft && onResetToSavedDraft) {
		actions.push({ label: 'Reset to saved draft', callback: onResetToSavedDraft })
	}
	if (hasDeployed && onResetToDeployed) {
		actions.push({ label: 'Reset to deployed', callback: onResetToDeployed })
	}
	if (actions.length === 0) return
	sendUserToast('Restored from local storage', false, actions)
}
