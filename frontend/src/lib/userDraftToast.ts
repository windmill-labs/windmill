/** Toast helpers for when a per-user draft is loaded from the server. */
import { sendUserToast } from '$lib/toast'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from '$lib/gen'

/**
 * Run a "reset to deployed" with autosave suspended: the reset's own
 * wipe-and-reload writes would otherwise re-create the draft just deleted.
 * Sync re-arms on the user's next interaction, not via `tick()` — editors
 * keep cascading writes after remount (Monaco acks, schema re-infer, iframe
 * acks) that would resurrect the draft. Shared by the load-time toast and
 * the AutosaveIndicator popover.
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
 * Post-deploy draft cleanup. Deletes the draft at its CANONICAL slot key
 * (the path the handle is keyed on, NOT the just-typed deploy path, which
 * differs for draft-only items) with sync muted and the delete flushed.
 * Mute + flush both matter: `remove` only QUEUES the delete in the
 * debouncer, and editors still mounted through the navigation (AppEditor,
 * RawAppEditor) keep mirroring their value — one such write displaces the
 * queued delete and re-saves the draft. Sync re-arms on next interaction.
 */
export function discardDraftAfterDeploy(opts: {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
}): void {
	UserDraft.stopSync(opts.itemKind, opts.path, { workspace: opts.workspace })
	UserDraft.remove(opts.itemKind, opts.path, { workspace: opts.workspace })
	void UserDraftDbSyncer.flush({
		workspace: opts.workspace,
		itemKind: opts.itemKind,
		path: opts.path
	})
	armRestartOnFirstInteraction(opts.workspace, opts.itemKind, opts.path)
}

/**
 * Defer `UserDraft.restartSync` to the first user interaction, with a 5s
 * fallback so the suspension can't leak when the user navigates away
 * untouched. Brackets bootstrap mutations that would otherwise POST as the
 * "first edit" (template fills, conflict reloads, reset-to-deployed); pair
 * with `UserDraft.stopSync` at the bootstrap start.
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
	// Three events: keydown misses pointer-driven UI (sliders, pickers, the
	// path popover OK); pointerdown misses pure-keyboard Monaco edits.
	document.addEventListener('keydown', restart, { capture: true, once: true })
	document.addEventListener('input', restart, { capture: true, once: true })
	document.addEventListener('pointerdown', restart, { capture: true, once: true })
	// Fallback if the user navigates away before interacting, else the
	// suspension leaks and autosave stays off next visit.
	const fallback = setTimeout(restart, 5000)
}
