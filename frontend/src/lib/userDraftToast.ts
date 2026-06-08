/**
 * Toast helpers shared by the editor routes when a per-user draft is loaded
 * from the server. Owns only the wording and reset action; the reset
 * side-effects live at each call site (route-specific state).
 */
import { sendUserToast } from '$lib/toast'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from '$lib/gen'

/**
 * Run a "reset to deployed" action with the reactive autosave mirror
 * suspended around it. The reset's own wipe-and-reload writes flow
 * through the cell and would otherwise re-create the draft we just
 * deleted; suspending swallows them silently.
 *
 * Restart sync ONLY on the user's next real interaction (keydown / input
 * / pointerdown on the document) — `tick()` alone wasn't enough because
 * editors keep cascading writes for a while after they remount (Monaco
 * setValue acks, schema re-infer, framework iframe acks, …) and any of
 * those landing post-restart would resurrect the draft. A 5-second
 * fallback re-arms sync if the user walks away without touching the
 * editor, so we don't leak the suspension forever.
 *
 * Exported so both the load-time toast (`notifyDraftLoaded`) and the
 * AutosaveIndicator popover share one implementation — keep them in
 * sync by funnelling through this helper.
 */
export async function runResetToDeployed(opts: {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	onResetToDeployed: () => void | Promise<void>
}): Promise<void> {
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
		armRestartOnFirstInteraction(opts.workspace, opts.itemKind, opts.path)
	}
}

/**
 * Defer `UserDraft.restartSync(workspace, itemKind, path)` to the first
 * user interaction (keydown / input / pointerdown on document, capture
 * phase). Falls back to a 5s timer if no interaction fires — without
 * that, the suspension would leak when the user navigates away without
 * touching the editor.
 *
 * Use to bracket bootstrap mutations that would otherwise POST as the
 * "first edit": new-draft template fills, conflict-modal reloads, the
 * reset-to-deployed dance. Pair with `UserDraft.stopSync` at the start
 * of the bootstrap.
 *
 * keydown alone misses pointer-driven UI (sliders, color pickers, the
 * Path popover's OK button); pointerdown alone misses pure-keyboard
 * edits in Monaco. Hence the three-event capture.
 */
export function armRestartOnFirstInteraction(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): void {
	let restarted = false
	const restart = () => {
		if (restarted) return
		restarted = true
		document.removeEventListener('keydown', restart, true)
		document.removeEventListener('input', restart, true)
		document.removeEventListener('pointerdown', restart, true)
		clearTimeout(fallback)
		UserDraft.restartSync(itemKind, path, { workspace })
	}
	// Capture phase + the three events that mean "the user touched the
	// editor". keydown alone misses pointer-driven UI (sliders, color
	// pickers, the path popover's OK button); pointerdown alone misses
	// pure-keyboard edits in Monaco. Belt-and-braces fallback after 5s
	// avoids stranding the suspension when the user navigates away.
	document.addEventListener('keydown', restart, { capture: true, once: true })
	document.addEventListener('input', restart, { capture: true, once: true })
	document.addEventListener('pointerdown', restart, { capture: true, once: true })
	// Belt-and-braces ceiling: if the user navigates away before
	// interacting, the suspension would otherwise leak and silently
	// turn off autosave on the next visit.
	const fallback = setTimeout(restart, 5000)
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
			callback: () =>
				runResetToDeployed({
					workspace: opts.workspace,
					itemKind: opts.itemKind,
					path: opts.path,
					onResetToDeployed: opts.onResetToDeployed
				})
		}
	])
}
