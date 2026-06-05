import { DraftService, type UserDraftItemKind } from './gen'
import { OpenAPI } from './gen/core/OpenAPI'
import { createCoalescingKeyedRunner } from './coalescingRunner.svelte'
import { createDebouncerByKey } from './debouncerByKey.svelte'

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
	/** Bypass the autosave debouncer for THIS save. Cancels any pending
	 * debouncer task for the same key (the queued autosave would
	 * otherwise overwrite what we're about to send), routes through the
	 * coalescing runner so ordering against any in-flight POST is
	 * preserved, and the returned promise resolves only after the POST
	 * actually lands. Use for `await save(...); then-read-the-server`
	 * flows (table-row delete, etc.) where a fire-and-forget save would
	 * race the next read. */
	immediate?: boolean
}

export type UserDraftLastSyncQuery = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
}

/**
 * Autosave lifecycle for a single draft, derived from the two-stage
 * pipeline:
 *   - `saving`:  a POST is currently in flight (coalescing runner busy).
 *   - `pending`: a change is queued in the debouncer but not yet fired.
 *   - `none`:    neither — the draft is in sync (or nothing happened).
 * `saving` wins over `pending` when both hold (a request for an earlier
 * change is in flight while a newer edit is still debouncing).
 */
export type UserDraftSyncState = 'none' | 'pending' | 'saving'

export type UserDraftStateHandle = {
	/** Reactive — read it inside a `$derived`/`$effect` and it re-runs as
	 * the draft moves through the pipeline. */
	readonly state: UserDraftSyncState
}

/**
 * Two-stage pipeline per draft key (`workspace/itemKind/path`):
 *   1. Debouncer collapses keystroke bursts so we don't POST per edit.
 *      `debounceMs = 1500` resets on each new submission; the
 *      `maxDebounceMs = 10000` ceiling guarantees sustained typing still
 *      flushes at least once every 10s instead of getting deferred
 *      forever.
 *   2. When the debouncer fires, the captured `opts` is submitted to a
 *      coalescing runner. The runner keeps at most one POST in flight
 *      per key plus at most one "latest" follow-up — newer submissions
 *      while a POST is running drop any prior pending and replace it,
 *      so the server never sees stale-then-fresh out of order.
 *
 * Together: the debouncer absorbs typing, the runner absorbs network
 * slowness. Imperative awaits (e.g. delete-then-refetch) MUST NOT rely
 * on `save()` returning a settled promise — the returned promise
 * resolves as soon as the work is queued, not when the POST lands. A
 * bypass path for those callers is tracked separately.
 */
const debouncer = createDebouncerByKey({ debounceMs: 1500, maxDebounceMs: 10000 })
const runner = createCoalescingKeyedRunner()

/**
 * Latest `save` opts per draft key that haven't been confirmed by a
 * successful `postSave`. The unload flush walks this map and fires a
 * `keepalive: true` POST for each entry so the browser commits the
 * request even after the document is torn down.
 *
 * Updated on every `save()` (newer opts displace older) and cleared by
 * `postSave` on a successful response — but only when the entry is
 * still the same object the success was for. A newer `save()` that
 * landed during the in-flight POST leaves its opts in the map so the
 * next round (or the flush) still picks them up.
 */
const pendingSaveOpts = new Map<string, UserDraftDbSyncerSaveOpts>()

async function postSave(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
	const key = draftKey(opts.workspace, opts.itemKind, opts.path)
	console.log('[draft-sync] postSave START (sending POST)', key, {
		valueIsNull: opts.value === null
	})
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
		// Clear pending only if it's still the opts we just saved — a
		// newer `save()` that arrived during the POST replaces the entry
		// and must survive for the next flush / debouncer round.
		if (pendingSaveOpts.get(key) === opts) pendingSaveOpts.delete(key)
		console.log('[draft-sync] postSave SUCCESS', key)
	} catch (e) {
		console.error('[draft-sync] postSave FAILED', key, e)
	}
}

/**
 * Fire `keepalive: true` POSTs for every unconfirmed save in
 * `pendingSaveOpts`. Browser-level guarantee: the request is committed
 * to the network stack and continues running after the document is
 * gone, so a tab close mid-debounce doesn't drop the in-flight edit.
 *
 * Trade-offs:
 *   - Total keepalive body size per page is capped (Chrome: 64KB). For
 *     huge editor states (low-code apps with many inline scripts) this
 *     may exceed the cap and the request will be rejected. We log and
 *     accept the loss for the oversized payload — still strictly better
 *     than the status quo, which drops every pending save.
 *   - We bypass the debouncer/runner pipeline because both are
 *     async-scheduled and won't get a chance to run after the document
 *     hides. The keepalive fetch is hand-rolled to mirror the generated
 *     client's URL/auth/body shape.
 *
 * Called on `visibilitychange → hidden` (most reliable signal across
 * browsers and mobile) and `pagehide` (belt-and-suspenders for navs
 * that bypass `visibilitychange`).
 */
function flushOnUnload(): void {
	if (pendingSaveOpts.size === 0) return
	for (const opts of pendingSaveOpts.values()) {
		try {
			// Path encoding mirrors the generated client (`encodeURI`,
			// not `encodeURIComponent`) so slashes inside the path
			// (e.g. `u/user/myScript`) pass through.
			const url =
				OpenAPI.BASE +
				`/w/${encodeURI(opts.workspace)}` +
				`/drafts/save_draft/${encodeURI(opts.itemKind)}` +
				`/${encodeURI(opts.path)}`
			void fetch(url, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ value: opts.value, force: true }),
				keepalive: true
			}).catch((e) => {
				// `keepalive` size cap or network error — log so the loss
				// is at least visible in devtools. We can't await this
				// during unload anyway.
				console.error('UserDraftDbSyncer: keepalive flush failed', e)
			})
		} catch (e) {
			console.error('UserDraftDbSyncer: keepalive flush threw', e)
		}
	}
	pendingSaveOpts.clear()
}

if (typeof document !== 'undefined') {
	document.addEventListener('visibilitychange', () => {
		if (document.visibilityState === 'hidden') flushOnUnload()
	})
	// `pagehide` covers full-document navs (link clicks, back/forward,
	// tab close) that don't always flip visibility — and is the
	// recommended last-line signal for "the page is going away".
	window.addEventListener('pagehide', flushOnUnload)
}

/**
 * Server-side persistence for `UserDraft`. Every write is funneled
 * through the debouncer + coalescing runner above so editor autosave
 * spam can't translate into one POST per keystroke or out-of-order
 * writes under slow networks.
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

	/**
	 * Reactive autosave state for a draft. Returns a handle whose `state`
	 * getter reads the debouncer / runner's reactive key-sets, so a Svelte
	 * consumer (`AutosaveIndicator`) can just `$derived(handle.state)` and
	 * re-render as the draft flows through pending → saving → none. The key
	 * is captured at call time; if the consumer's `(workspace, itemKind,
	 * path)` can change, recompute the handle in a `$derived`.
	 */
	getState(query: UserDraftLastSyncQuery): UserDraftStateHandle {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		return {
			get state(): UserDraftSyncState {
				if (runner.isRunning(key)) return 'saving'
				if (debouncer.isPending(key)) return 'pending'
				return 'none'
			}
		}
	},

	async save(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
		const key = draftKey(opts.workspace, opts.itemKind, opts.path)
		console.log('[draft-sync] UserDraftDbSyncer.save called', key, {
			immediate: !!opts.immediate,
			valueIsNull: opts.value === null
		})
		// Track the latest unconfirmed save BEFORE entering the pipeline
		// so the unload flush has something to send even if the page
		// hides before the debouncer fires.
		pendingSaveOpts.set(key, opts)
		if (opts.immediate) {
			// Drop the queued autosave (if any) — letting it fire after
			// our POST would re-save the pre-delete value. The runner's
			// own cancel is implicit in `submitAndWait`, which displaces
			// any pending runner task with ours, but call it explicitly
			// so a `submit` -> `cancel` -> `submitAndWait` sequence is
			// observable in the runner's internal state for debugging.
			debouncer.cancel(key)
			runner.cancel(key)
			await runner.submitAndWait(key, () => postSave(opts))
			return
		}
		debouncer.schedule(key, () => {
			runner.submit(key, () => postSave(opts))
		})
	}
}
