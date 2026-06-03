/**
 * "Restored from local storage" toast, shown when an editor reopens on a
 * local autosave that differs from the backend. Owns only the wording and
 * which reset actions are offered; the reset side-effects live at each call
 * site (route-specific state).
 */
import { sendUserToast } from '$lib/toast'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from '$lib/gen'

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

/**
 * Shown when an editor mounts on a per-user draft fetched from the server
 * (the `get_draft=true` overlay returned `is_draft: true`). Offers a
 * "Reset to deployed" action that:
 *   1. POSTs `value: null` to delete the user's draft for this entity, and
 *   2. invokes `onResetToDeployed` so the editor can re-fetch without the
 *      overlay and apply the pure deployed version.
 *
 * Failures in either step surface a follow-up toast and leave the editor
 * state untouched.
 */
export function notifyDraftLoaded(opts: {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	onResetToDeployed: () => void | Promise<void>
}): void {
	sendUserToast('Loaded your saved draft', false, [
		{
			label: 'Reset to deployed',
			callback: async () => {
				try {
					await UserDraftDbSyncer.save({
						workspace: opts.workspace,
						itemKind: opts.itemKind,
						path: opts.path,
						value: null
					})
				} catch (e: any) {
					sendUserToast(`Could not delete draft: ${e?.body ?? e}`, true)
					return
				}
				try {
					await opts.onResetToDeployed()
				} catch (e: any) {
					sendUserToast(`Could not reset to deployed: ${e?.body ?? e}`, true)
				}
			}
		}
	])
}
