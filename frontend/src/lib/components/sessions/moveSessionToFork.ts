import { tick } from 'svelte'
import { DraftService, type DraftItemRef } from '$lib/gen'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
import { invalidateWorkspaceDiffCache } from '$lib/components/copilot/chat/global/diffSnapshot'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import { flushPendingKeystrokes } from '$lib/components/editorKeystrokeFlush'
import { flushEditorSaves } from './useUserDraftSync.svelte'
import { materializeFork, moveSessionToWorkspace, type PendingFork } from './sessionState.svelte'

export type MoveTarget = { kind: 'existing'; id: string } | { kind: 'new'; fork: PendingFork }

export type MoveSessionResult =
	| { ok: true; forkId: string; copied: number; removed: number; warning?: string }
	| { ok: false; error: string; conflicts?: DraftItemRef[] }

/**
 * Send every debounced draft save for `workspace` to the server. Returns an error message
 * when one of them failed or conflicted.
 *
 * A save still debounced in an open editor would otherwise land in the parent after the
 * move: re-creating a draft just removed there, or missing from the copy. Three debounces
 * stand between a keystroke and the server, and each hands on to the next through an
 * effect, hence the ticks.
 */
export async function flushPendingSaves(workspace: string): Promise<string | undefined> {
	flushPendingKeystrokes()
	await tick()
	flushEditorSaves(workspace)
	await tick()
	const unsaved = await UserDraftDbSyncer.flushWorkspace(workspace)
	if (unsaved.length > 0) {
		return `Could not save ${unsaved.map((q) => q.path).join(', ')} before moving. Resolve it and retry.`
	}
}

/**
 * Move a session into a fork of its workspace, bringing its drafts along. The session's
 * workspace changes only once its drafts are where they must be, so a failure at any step
 * leaves it acting on the parent with its drafts intact.
 *
 * A new fork already receives every draft of the user through the fork clone, so only an
 * existing fork needs the copy. Once a new fork exists the move always completes: a retry
 * would adopt that fork without cloning again, and then remove parent drafts saved after the
 * clone. Removing the parent's copies is therefore best-effort there, reported as `warning`.
 */
export async function moveSessionToFork(
	sessionId: string,
	parent: string,
	target: MoveTarget,
	opts: {
		items: DraftItemRef[]
		removeFromParent: boolean
		// A turn running in the parent keeps writing there while its drafts move away.
		isBusy?: () => boolean
	}
): Promise<MoveSessionResult> {
	const busy = { ok: false as const, error: 'A chat turn is running. Retry once it ends.' }
	const unsaved = await flushPendingSaves(parent)
	if (unsaved) return { ok: false, error: unsaved }

	if (opts.isBusy?.()) return busy

	let forkId: string
	let copied = 0
	let removed = 0
	let warning: string | undefined
	try {
		if (target.kind === 'new') {
			const created = await materializeFork(target.fork)
			if (!created) return { ok: false, error: 'Could not create the fork' }
			forkId = created
			copied = opts.items.length
			if (opts.removeFromParent && opts.items.length > 0) {
				try {
					const res = await DraftService.transferDrafts({
						workspace: parent,
						requestBody: { items: opts.items, remove_from_source: true }
					})
					removed = res.removed.length
				} catch (e: any) {
					warning = `The drafts are in the fork but could not be removed from ${parent}: ${e?.body ?? e?.message ?? e}`
				}
			}
		} else {
			forkId = target.id
			if (opts.items.length > 0) {
				const res = await DraftService.transferDrafts({
					workspace: parent,
					requestBody: {
						target_workspace: forkId,
						items: opts.items,
						remove_from_source: opts.removeFromParent
					}
				})
				if (res.conflicts.length > 0) {
					return {
						ok: false,
						error: 'You already have drafts of these items in the fork',
						conflicts: res.conflicts
					}
				}
				copied = res.copied.length
				removed = res.removed.length
			}
		}
	} catch (e: any) {
		return { ok: false, error: String(e?.body ?? e?.message ?? e) }
	}

	if (removed > 0) {
		for (const it of opts.items) UserDraft.forgetLocal(it.kind, it.path, { workspace: parent })
	}
	await moveSessionToWorkspace(sessionId, forkId)
	for (const ws of [parent, forkId]) {
		invalidateWorkspaceDrafts(ws)
		invalidateWorkspaceDiffCache(ws)
	}
	logFeatureUsage('ai_session', 'moved_to_fork', {
		key: opts.removeFromParent ? 'copy_and_clean_parent' : 'copy',
		entityId: sessionId,
		workspace: forkId
	})
	return { ok: true, forkId, copied, removed, warning }
}
