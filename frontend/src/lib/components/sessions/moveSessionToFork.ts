import { tick } from 'svelte'
import { DraftService, type DraftItemRef } from '$lib/gen'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
import { invalidateWorkspaceDiffCache } from '$lib/components/copilot/chat/global/diffSnapshot'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import { flushPendingKeystrokes } from '$lib/components/editorKeystrokeFlush'
import { flushEditorSaves } from './useUserDraftSync.svelte'
import {
	materializeForkOutcome,
	moveSessionToWorkspace,
	type PendingFork
} from './sessionState.svelte'

/**
 * `replace` marks a fork this move created itself, whose own drafts at the moved paths are the
 * fork clone's copies of them: they are overwritten rather than reported as conflicts.
 */
export type MoveTarget =
	| { kind: 'existing'; id: string; replace?: boolean }
	| { kind: 'new'; fork: PendingFork }

export type MoveSessionResult =
	| { ok: true; forkId: string; copied: number; removed: number }
	// `retry` is the target to retry with once the fork exists, so a retry never creates it again.
	| { ok: false; error: string; conflicts?: DraftItemRef[]; retry?: MoveTarget }

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
 * Load the moved drafts into editors already open on them in `workspace`. Such an editor
 * still holds what it loaded before the move, with no baseline to conflict on, so its next
 * save would overwrite the moved draft.
 */
async function reloadOpenEditors(workspace: string, items: DraftItemRef[]): Promise<void> {
	await Promise.all(
		items
			.filter((it) => UserDraft.isLive(it.kind, it.path, { workspace }))
			.map(async (it) => {
				const draft = await DraftService.getOwnDraft({
					workspace,
					kind: it.kind,
					path: it.path
				}).catch(() => undefined)
				if (draft?.value == null) return
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace, itemKind: it.kind, path: it.path },
					draft.created_at
				)
				UserDraft.seed(it.kind, it.path, draft.value, { workspace })
			})
	)
}

/**
 * Move a session into a fork of its workspace, bringing its drafts along. The session's
 * workspace changes only once its drafts are where they must be, so a failure at any step
 * leaves it acting on the parent with its drafts intact.
 *
 * A new fork already holds the user's drafts through the fork clone, but a save made after the
 * clone is only in the parent, so the drafts are copied again, under the same lock as their
 * removal from the parent.
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
	const unsaved = await flushPendingSaves(parent)
	if (unsaved) return { ok: false, error: unsaved }
	if (opts.isBusy?.()) return { ok: false, error: 'A chat turn is running. Retry once it ends.' }

	let fork: Extract<MoveTarget, { kind: 'existing' }>
	if (target.kind === 'new') {
		const made = await materializeForkOutcome(target.fork)
		if (!made) return { ok: false, error: 'Could not create the fork' }
		fork = { kind: 'existing', id: made.id, replace: made.created }
	} else {
		fork = target
	}

	let copied = 0
	let removed = 0
	if (opts.items.length > 0) {
		try {
			const res = await DraftService.transferDrafts({
				workspace: parent,
				requestBody: {
					target_workspace: fork.id,
					items: opts.items,
					remove_from_source: opts.removeFromParent,
					replace_existing: fork.replace
				}
			})
			if (res.conflicts.length > 0) {
				return {
					ok: false,
					error: 'You already have drafts of these items in the fork',
					conflicts: res.conflicts,
					retry: fork
				}
			}
			copied = res.copied.length
			removed = res.removed.length
		} catch (e: any) {
			return { ok: false, error: String(e?.body ?? e?.message ?? e), retry: fork }
		}
	}

	if (removed > 0) {
		for (const it of opts.items) UserDraft.forgetLocal(it.kind, it.path, { workspace: parent })
	}
	await reloadOpenEditors(fork.id, opts.items)
	await moveSessionToWorkspace(sessionId, fork.id)
	for (const ws of [parent, fork.id]) {
		invalidateWorkspaceDrafts(ws)
		invalidateWorkspaceDiffCache(ws)
	}
	logFeatureUsage('ai_session', 'moved_to_fork', {
		key: opts.removeFromParent ? 'copy_and_clean_parent' : 'copy',
		entityId: sessionId,
		workspace: fork.id
	})
	return { ok: true, forkId: fork.id, copied, removed }
}
