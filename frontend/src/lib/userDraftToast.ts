/**
 * "Restored from local storage" toast, shown when an editor reopens on a
 * local autosave that differs from the backend. Owns only the wording and
 * which reset actions are offered; the reset side-effects live at each call
 * site (route-specific state).
 */
import { tick } from 'svelte'
import { sendUserToast } from '$lib/toast'
import { UserDraft } from '$lib/userDraft.svelte'
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
 * (the `get_draft=true` overlay returned `is_draft: true`).
 *
 * Offers a "Reset to deployed" action UNLESS `draftOnly` is set — when
 * the response came back via `fetch_draft_only` there is no deployed row
 * at this path, so falling back has nothing to fall back to. In that
 * case we still show the toast (the user should know they're editing a
 * draft) but omit the action.
 *
 * The action:
 *   1. Fires `value: null` at the syncer (fire-and-forget — the user's
 *      delete races no reader; `onResetToDeployed` doesn't depend on it).
 *   2. invokes `onResetToDeployed`, which MUST fetch deployed directly
 *      (e.g. `getDraft: false`) instead of trusting that the delete has
 *      already landed in the DB.
 *
 * A delete failure logs to console only — there's no UI state that
 * depends on it. A reset-callback failure surfaces a toast.
 */
export function notifyDraftLoaded(opts: {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	draftOnly?: boolean
	onResetToDeployed: () => void | Promise<void>
}): void {
	if (opts.draftOnly) {
		sendUserToast('Loaded your saved draft')
		return
	}
	sendUserToast('Loaded your saved draft', false, [
		{
			label: 'Reset to deployed',
			callback: async () => {
				// Suspend the reactive sync effect around the whole reset
				// flow: the route's callback wipes the in-memory handle
				// then re-seeds it with the deployed payload, both of
				// which would otherwise fire the autosave mirror and
				// resurrect the draft we just deleted. We restart sync
				// only after two ticks so the deployed-seed write has
				// observably advanced `lastSerialized` first.
				UserDraft.stopSync(opts.itemKind, opts.path, { workspace: opts.workspace })
				UserDraftDbSyncer.save({
					workspace: opts.workspace,
					itemKind: opts.itemKind,
					path: opts.path,
					value: null
				}).catch((e) => console.error('Reset to deployed: draft delete failed', e))
				try {
					await opts.onResetToDeployed()
				} catch (e: any) {
					sendUserToast(`Could not reset to deployed: ${e?.body ?? e}`, true)
				} finally {
					await tick()
					await tick()
					UserDraft.restartSync(opts.itemKind, opts.path, { workspace: opts.workspace })
				}
			}
		}
	])
}
