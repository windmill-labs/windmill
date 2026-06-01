import { DraftService, type UserDraftItemKind } from './gen'

/**
 * Per-draft last-sync map persisted under a single localStorage key. Lets
 * the next `save_draft` (or a downstream UI) know the server's clock at
 * our most recent successful sync for `(workspace, itemKind, path)` —
 * without doing a network round-trip first.
 *
 * Shape: `Record<draftKey, { lastSync: ISO-8601 string }>`. The map is
 * intentionally one storage slot for the whole tab; reading is one
 * `localStorage.getItem` + `JSON.parse`, not one per entry.
 */
const DRAFT_LAST_SYNC_KEY = 'userdraft/draftLastSync'

type DraftLastSyncEntry = { lastSync: string }
type DraftLastSyncMap = Record<string, DraftLastSyncEntry>

/** Must match `mapKey` in `userDraft.svelte.ts` so the two files agree
 *  on what identifies a draft. */
function draftKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

function readLastSyncMap(): DraftLastSyncMap {
	try {
		const raw = localStorage.getItem(DRAFT_LAST_SYNC_KEY)
		if (!raw) return {}
		const parsed = JSON.parse(raw)
		// Defensive: a corrupt slot (wrong type, array, null) shouldn't
		// crash the syncer — reset to an empty map.
		if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
			return parsed as DraftLastSyncMap
		}
		return {}
	} catch (e) {
		console.error('UserDraftDbSyncer: draftLastSync read failed', e)
		return {}
	}
}

function writeLastSyncMap(map: DraftLastSyncMap): void {
	try {
		localStorage.setItem(DRAFT_LAST_SYNC_KEY, JSON.stringify(map))
	} catch (e) {
		console.error('UserDraftDbSyncer: draftLastSync write failed', e)
	}
}

function setLastSync(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	lastSync: string
): void {
	// Read-modify-write is fine: each step is synchronous, so two saves
	// for different keys can't interleave their updates within a tab.
	const map = readLastSyncMap()
	map[draftKey(workspace, itemKind, path)] = { lastSync }
	writeLastSyncMap(map)
}

function clearLastSync(workspace: string, itemKind: UserDraftItemKind, path: string): void {
	const map = readLastSyncMap()
	delete map[draftKey(workspace, itemKind, path)]
	writeLastSyncMap(map)
}

export type UserDraftDbSyncerSaveOpts = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	/** `null` signals a delete — the server removes the row under the same
	 *  conflict rules as an upsert. */
	value: unknown | null
}

export type UserDraftLastSyncQuery = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
}

/**
 * Server-side persistence for `UserDraft`. `UserDraft` owns the localStorage
 * cache; this module forwards each write through to `POST /drafts/save_draft`
 * so the per-user `draft` table on the server stays in sync.
 *
 * Kept as a separate module so the two halves stay decoupled — `UserDraft`
 * just calls `UserDraftDbSyncer.save(...)` and doesn't reach into the
 * generated client. Adding conflict handling (`last_sync` + a reject UI)
 * later means changing this file, not every editor.
 *
 * NOTE: every save currently uses `force: true`, so the server copy is
 * unconditionally overwritten. This is intentional for the first cut —
 * we'll thread `last_sync` through (using `getLastSync` below) once the
 * client side is settled.
 */
export const UserDraftDbSyncer = {
	/**
	 * Read the recorded last-sync timestamp (ISO string) for a draft.
	 * Returns `undefined` when we've never successfully synced this
	 * `(workspace, itemKind, path)` — a fresh save should be treated as
	 * a first-time push.
	 */
	getLastSync(opts: UserDraftLastSyncQuery): string | undefined {
		return readLastSyncMap()[draftKey(opts.workspace, opts.itemKind, opts.path)]?.lastSync
	},

	async save(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
		try {
			const resp = await DraftService.saveDraft({
				workspace: opts.workspace,
				kind: opts.itemKind,
				path: opts.path,
				requestBody: {
					value: opts.value as any,
					force: true
				}
			})
			// On a successful delete, drop the recorded last-sync so the
			// next save starts fresh. On an upsert, remember the server's
			// timestamp as our baseline for future conflict checks.
			if (opts.value === null) {
				clearLastSync(opts.workspace, opts.itemKind, opts.path)
			} else {
				setLastSync(opts.workspace, opts.itemKind, opts.path, resp.current_timestamp)
			}
		} catch (e) {
			console.error('UserDraftDbSyncer.save failed', e)
		}
	}
}
