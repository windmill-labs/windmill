/**
 * Bi-directional sync layer between the local `UserDraft` autosave and the
 * server-side `draft` table. The transport is `DraftService.syncDrafts`,
 * which doubles as "what the server has for me that I haven't seen yet" and
 * "push these drafts up, rejecting any whose server copy moved forward
 * since my last sync".
 *
 * The syncer is a single shared module-level service (not per-handle) so
 * that pushes from any UserDraft.save call across the app coalesce into one
 * batched sync request.
 */
import type { UserDraftItemKind } from './userDraft.svelte'
import { DraftService, type SyncDraftsResponse } from './gen'
import { useLocalStorageValue } from './svelte5Utils.svelte'

const LAST_SYNC_KEY = 'userdraft/lastSync'
const DEBOUNCE_MS = 2_000
const MAX_DEBOUNCE_MS = 10_000

export type SyncableItemKind = 'script' | 'flow' | 'app'

/**
 * UserDraft item kinds that have a matching backend `draft.typ` enum value.
 * Other kinds (resources, variables, individual triggers...) only exist in
 * localStorage for now and are filtered out of every push.
 */
const SYNCABLE_KINDS: ReadonlySet<UserDraftItemKind> = new Set(['script', 'flow', 'app'])

export function isSyncableKind(kind: UserDraftItemKind): kind is SyncableItemKind {
	return SYNCABLE_KINDS.has(kind)
}

export type MissedDraft = SyncDraftsResponse['missed_drafts'][number]
export type RejectedDraft = Extract<SyncDraftsResponse['statuses'][number], { status: 'rejected' }>

export type PendingDraft<V = unknown> = {
	itemKind: SyncableItemKind
	path: string
	value: V
}

export type MissedDraftCallback = (drafts: MissedDraft[]) => void
export type RejectedDraftsCallback = (rejected: RejectedDraft[]) => void

export type SyncOptions<V = unknown> = {
	workspace: string
	user: string
	drafts: PendingDraft<V>[]
	force?: boolean
	onMissedDrafts?: MissedDraftCallback
	onDraftsRejected?: RejectedDraftsCallback
}

type QueueKey = string
function queueKey(workspace: string, user: string, kind: SyncableItemKind, path: string): QueueKey {
	return `${workspace}|${user}|${kind}|${path}`
}

type QueuedEntry = {
	workspace: string
	user: string
	itemKind: SyncableItemKind
	path: string
	value: unknown
	force: boolean
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

const queue = new Map<QueueKey, QueuedEntry>()
let debounceTimer: ReturnType<typeof setTimeout> | undefined
let maxDebounceTimer: ReturnType<typeof setTimeout> | undefined

function clearTimers(): void {
	if (debounceTimer !== undefined) {
		clearTimeout(debounceTimer)
		debounceTimer = undefined
	}
	if (maxDebounceTimer !== undefined) {
		clearTimeout(maxDebounceTimer)
		maxDebounceTimer = undefined
	}
}

async function flushQueue(): Promise<void> {
	clearTimers()
	if (queue.size === 0) return

	const entries = Array.from(queue.values())
	queue.clear()

	// Group entries by (workspace, user) — every sync call is scoped to a
	// single user, so we issue one request per distinct group. In practice
	// the queue is dominated by the active session's workspace+user, so
	// there's almost always exactly one group.
	const groups = new Map<string, QueuedEntry[]>()
	for (const entry of entries) {
		const key = `${entry.workspace}|${entry.user}`
		const list = groups.get(key)
		if (list) list.push(entry)
		else groups.set(key, [entry])
	}

	for (const group of groups.values()) {
		await runSync(group[0].workspace, group)
	}
}

async function runSync(workspace: string, group: QueuedEntry[]): Promise<void> {
	const onMissedDrafts = group.find((e) => e.onMissedDrafts)?.onMissedDrafts
	const onDraftsRejected = group.find((e) => e.onDraftsRejected)?.onDraftsRejected
	const force = group.some((e) => e.force)
	const drafts: PendingDraft[] = group.map(({ itemKind, path, value }) => ({
		itemKind,
		path,
		value
	}))
	await syncDrafts({
		workspace,
		user: group[0].user,
		drafts,
		force,
		onMissedDrafts,
		onDraftsRejected
	})
}

/**
 * Immediate sync. Bypasses the queue. Caller is responsible for handling
 * the missed/rejected lists.
 */
export async function syncDrafts<V = unknown>(opts: SyncOptions<V>): Promise<void> {
	const lastSync = getLastSync()
	const payloadDrafts = opts.drafts.map((d) => ({
		path: d.path,
		typ: d.itemKind,
		value: d.value as any
	}))
	const result = await DraftService.syncDrafts({
		workspace: opts.workspace,
		requestBody: {
			last_sync: lastSync,
			drafts: payloadDrafts,
			force: opts.force ?? false
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

	/**
	 * Test-only: clear the in-memory queue + lastSync slot so successive
	 * tests start with a clean slate.
	 */
	__resetForTesting(): void {
		clearTimers()
		queue.clear()
		lastSyncCell.val = undefined
	},

	/**
	 * Enqueue drafts for a batched sync. Repeated pushes for the same
	 * (workspace, user, itemKind, path) coalesce — only the latest value /
	 * callbacks survive. Returns the same Promise as the eventual
	 * `syncDrafts` so callers can `await` flush completion.
	 */
	pushDrafts<V = unknown>(opts: SyncOptions<V>): void {
		const ws = opts.workspace
		const user = opts.user
		for (const d of opts.drafts) {
			queue.set(queueKey(ws, user, d.itemKind, d.path), {
				workspace: ws,
				user,
				itemKind: d.itemKind,
				path: d.path,
				value: d.value,
				force: opts.force ?? false,
				onMissedDrafts: opts.onMissedDrafts,
				onDraftsRejected: opts.onDraftsRejected
			})
		}

		if (debounceTimer !== undefined) clearTimeout(debounceTimer)
		debounceTimer = setTimeout(() => {
			void flushQueue()
		}, DEBOUNCE_MS)

		if (maxDebounceTimer === undefined) {
			maxDebounceTimer = setTimeout(() => {
				void flushQueue()
			}, MAX_DEBOUNCE_MS)
		}
	},

	/**
	 * Force a synchronous flush of any queued pushes. Useful for tests and
	 * for "logout" / "navigate away" hooks that want to guarantee delivery.
	 */
	async flush(): Promise<void> {
		await flushQueue()
	}
}
