/**
 * Module-level queue of UserDraft sync conflicts (drafts rejected by the
 * server because a newer version was saved since the client's last sync).
 *
 * Components anywhere in the app can call `enqueueConflicts()` to surface a
 * conflict; the single `UserDraftConflictModal` mounted in the root layout
 * consumes them one at a time so the user can decide per-conflict.
 */
import type { RejectedDraft, SyncableItemKind } from './userDraftDbSyncer.svelte'

export type ConflictEntry = {
	workspace: string
	user: string
	itemKind: SyncableItemKind
	rejected: RejectedDraft
}

let pending = $state<ConflictEntry[]>([])

export const UserDraftConflictStore = {
	get current(): ConflictEntry | undefined {
		return pending[0]
	},
	get hasAny(): boolean {
		return pending.length > 0
	},
	enqueue(entries: ConflictEntry[]): void {
		if (entries.length === 0) return
		pending.push(...entries)
	},
	dismiss(): void {
		pending.shift()
	},
	clear(): void {
		pending.length = 0
	}
}
