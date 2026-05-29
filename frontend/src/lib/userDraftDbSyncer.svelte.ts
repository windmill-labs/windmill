/**
 * Bi-directional sync layer between the local `UserDraft` autosave and the
 * server-side `draft` table. The transport is `DraftService.syncDrafts`,
 * which doubles as "what the server has for me that I haven't seen yet" and
 * "push these drafts up, rejecting any whose server copy moved forward
 * since my last sync".
 */
import type { UserDraftItemKind } from './userDraft.svelte'
import { DraftService, type SyncDraftsResponse } from './gen'
import { useLocalStorageValue } from './svelte5Utils.svelte'

const LAST_SYNC_KEY = 'userdraft/lastSync'

export type MissedDraft = SyncDraftsResponse['missed_drafts'][number]
export type RejectedDraft = Extract<SyncDraftsResponse['statuses'][number], { status: 'rejected' }>

export type PendingDraft<V = unknown> = {
	itemKind: UserDraftItemKind
	path: string
	/**
	 * Draft content. `null` (or omitted) signals a delete — the server
	 * removes the row at this path, applying the same conflict semantics
	 * as an upsert.
	 */
	value: V | null
	/**
	 * Skip the conflict check for this single entry and overwrite the
	 * server copy. Used by the conflict-resolution modal's "Overwrite
	 * server draft" / "Delete anyway" actions; routine autosaves leave
	 * this `false`.
	 */
	force?: boolean
}

export type MissedDraftCallback = (drafts: MissedDraft[]) => void
export type RejectedDraftsCallback = (rejected: RejectedDraft[]) => void

export type SyncOptions<V = unknown> = {
	workspace: string
	drafts: PendingDraft<V>[]
	onMissedDrafts?: MissedDraftCallback
	onDraftsRejected?: RejectedDraftsCallback
}

// Setter-only callers can use `useLocalStorageValue` at module scope by
// disabling the nested-mutation `$effect`. The lastSync slot is a flat
// string updated exclusively via `cell.val = ...`, so the effect is
// unnecessary.
const lastSyncCell = useLocalStorageValue<string | undefined>(LAST_SYNC_KEY, undefined, 'string', {
	saveInitialValue: false
})

function getLastSync(): string | undefined {
	return lastSyncCell.val
}

function bumpLastSync(serverTimestamp: string): void {
	const previous = lastSyncCell.val
	if (!previous || new Date(serverTimestamp).getTime() > new Date(previous).getTime()) {
		lastSyncCell.val = serverTimestamp
	}
}

/**
 * Immediate sync. Caller is responsible for handling the missed/rejected
 * lists via the callbacks.
 */
export async function syncDrafts<V = unknown>(opts: SyncOptions<V>): Promise<void> {
	const lastSync = getLastSync()
	const payloadDrafts = opts.drafts.map((d) => ({
		path: d.path,
		typ: d.itemKind,
		value: d.value as any,
		force: d.force ?? false
	}))
	const result = await DraftService.syncDrafts({
		workspace: opts.workspace,
		requestBody: {
			last_sync: lastSync,
			drafts: payloadDrafts
		}
	})
	bumpLastSync(result.current_timestamp as unknown as string)

	if (result.missed_drafts.length > 0 && opts.onMissedDrafts) {
		opts.onMissedDrafts(result.missed_drafts)
	}

	const rejected = result.statuses.filter((s): s is RejectedDraft => s.status === 'rejected')
	if (rejected.length > 0 && opts.onDraftsRejected) {
		opts.onDraftsRejected(rejected)
	}
}

export const UserDraftDbSyncer = {
	getLastSync,
}
