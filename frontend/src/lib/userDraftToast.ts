import { sendUserToast } from '$lib/toast'

export type RestoreFromLocalActions = {
	/**
	 * Caller's "reset to saved draft" logic — drop the local autosave and
	 * apply the backend DB draft. Only included in the toast when
	 * `hasBackendDraft` is true.
	 */
	onResetToSavedDraft?: () => void | Promise<void>
	/**
	 * Caller's "reset to deployed" logic — drop the local autosave (and,
	 * typically, the backend DB draft) and load the deployed version. Only
	 * included in the toast when `hasDeployed` is true.
	 */
	onResetToDeployed?: () => void | Promise<void>
}

/**
 * Fired by each editor route when its initial state was rehydrated from a
 * local autosave that differs from the backend. The user gets up to two
 * reset options — back to the saved DB draft and / or back to the deployed
 * version — depending on what the backend has.
 *
 * Centralises the toast text + label wording + per-state inclusion logic so
 * the four editors that care (script/flow/app/raw_app) stay in sync. The
 * actual reset side-effects live at the call site because each editor
 * touches different state (UserDraft handle, in-memory $state, redraw
 * counters, loadXxx helpers).
 */
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
