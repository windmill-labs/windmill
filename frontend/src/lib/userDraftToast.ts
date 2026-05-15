/**
 * Toast used by the four route-level editors (scripts, flows, apps,
 * raw_apps) when they reopen on a local autosave that diverges from the
 * backend. Sits at the layer above `UserDraft` (the per-browser autosave
 * store) and is separate from the backend's `DraftService` (the
 * server-side "saved draft" feature, surfaced as `script.draft` /
 * `flow.draft` / `app.draft`):
 *
 * - DraftService writes go through a "Save as draft" button and produce a
 *   shared, server-persisted draft visible to other users / tabs / CLI.
 * - UserDraft writes are this-browser autosave for in-flight edits,
 *   debounced into localStorage by `useLocalStorageValue`.
 *
 * The toast lets the user reconcile their local autosave with the two
 * server states (saved draft + deployed version) by offering up to two
 * reset actions. The actual reset side-effects live at each call site
 * because they touch route-specific state (UserDraft handle, redraw
 * counters, loadXxx helpers); this helper just owns the toast title +
 * label wording + per-state inclusion logic.
 */
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
