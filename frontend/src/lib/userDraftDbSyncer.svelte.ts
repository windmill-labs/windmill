import { SvelteMap } from 'svelte/reactivity'
import { DraftService, type UserDraftItemKind } from './gen'
import { OpenAPI } from './gen/core/OpenAPI'
import { createCoalescingKeyedRunner } from './coalescingRunner.svelte'
import { createDebouncerByKey } from './debouncerByKey.svelte'

/**
 * Per-draft last-sync map. Lets the next `save_draft` know the server's
 * clock at our most recent successful sync for `(workspace, itemKind,
 * path)` so it can attach `last_sync` and the backend can reject stale
 * writes.
 *
 * Lives in TAB-LOCAL memory — not localStorage — because the whole point
 * is that two tabs each track THEIR OWN baseline. If we shared the map,
 * tab-1's successful save would update tab-2's "last_sync" to tab-1's
 * fresh timestamp, and the next tab-2 save would attach that newer
 * timestamp, the server's WHERE clause (`created_at <= last_sync`) would
 * be true, and tab-2 would clobber tab-1's edit. That's the conflict
 * detection failure mode the whole feature is meant to prevent.
 *
 * The cost of being tab-local: a fresh tab reload starts with no
 * `last_sync`, so its first save sends nothing → the backend's "treat as
 * fresh" branch fires and the save lands unconditionally. That's
 * acceptable — the editor's load path calls `recordRemoteSync(query,
 * draft_saved_at)` right after `get_draft=true` returns, which reseeds
 * the map from the authoritative server timestamp before any user edit
 * could fire a save.
 */
type DraftLastSyncEntry = { lastSync: string }
const lastSyncMap = new Map<string, DraftLastSyncEntry>()

/** Must match `mapKey` in `userDraft.svelte.ts` so the two files agree
 *  on what identifies a draft. */
function draftKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

function getLastSyncEntry(key: string): DraftLastSyncEntry | undefined {
	return lastSyncMap.get(key)
}

function setLastSync(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	lastSync: string
): void {
	lastSyncMap.set(draftKey(workspace, itemKind, path), { lastSync })
}

function clearLastSync(workspace: string, itemKind: UserDraftItemKind, path: string): void {
	lastSyncMap.delete(draftKey(workspace, itemKind, path))
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
	/** Skip the optimistic-concurrency check on this save and overwrite
	 *  the server row unconditionally. Used by the conflict-resolution UI
	 *  ("Overwrite the remote") and by callers that have already resolved
	 *  the conflict locally. Default `false`: regular autosaves attach
	 *  `last_sync` and respect the server's reject response. */
	force?: boolean
}

/**
 * Snapshot of a rejected save. `localLastSync` is what we sent the
 * server (or `null` if we'd never synced this key); `serverTimestamp` is
 * the row's current `created_at`, surfaced so the resolution UI can show
 * how recent the conflicting write was.
 */
export type DraftConflictInfo = {
	serverTimestamp: string
	localLastSync: string | null
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
 *   - `failed`:  the last POST threw (network / 5xx / etc.) and no later
 *                attempt has succeeded. Conflicts use the conflict modal
 *                path and don't surface here.
 *   - `none`:    none of the above — the draft is in sync (or nothing
 *                happened).
 * Priority on render: `saving` > `pending` > `failed` > `none`. An active
 * save attempt outranks the prior failure so the indicator shows the
 * retry as in-flight; once it settles, either success clears `failed` or
 * a fresh throw sets it again.
 */
export type UserDraftSyncState = 'none' | 'pending' | 'saving' | 'failed'

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

/**
 * Reactive map of conflict snapshots — populated when the server rejects
 * a save because the row's `created_at` is newer than the `last_sync` we
 * sent. Read via `getConflict(query)` from the UI to drive the
 * resolution modal. SvelteMap so consumer `$derived` re-runs when an
 * entry is added/removed.
 */
const conflicts = new SvelteMap<string, DraftConflictInfo>()

/**
 * Reactive set of draft keys whose last save attempt threw (network
 * error, 5xx, anything `DraftService.saveDraft` rejects with). Cleared
 * on the next successful save for that key. Drives the
 * AutosaveIndicator's "Save failed" label so a silent failure doesn't
 * masquerade as "Saved" — the previous behaviour was to console.error
 * and let the runner finish, which the indicator read as success.
 * Conflicts are tracked separately (DraftSyncConflictModal handles
 * those) and don't populate this map.
 */
const failures = new SvelteMap<string, true>()

async function postSave(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
	const key = draftKey(opts.workspace, opts.itemKind, opts.path)
	const lastSync = getLastSyncEntry(key)?.lastSync
	try {
		const resp = await DraftService.saveDraft({
			workspace: opts.workspace,
			kind: opts.itemKind,
			path: opts.path,
			requestBody: {
				value: opts.value as any,
				// Force-saves skip the conflict check (callers that
				// explicitly opted in via `overwrite(...)` already showed
				// the user the conflict). Regular autosaves attach
				// `last_sync` (when we have one) so the server can reject
				// stale writes — defining behaviour for a first-ever save
				// is "no last_sync field at all", matching the backend's
				// "treat as fresh" branch.
				last_sync: opts.force ? undefined : lastSync,
				force: opts.force ?? false
			}
		})
		if (resp.status === 'conflict') {
			// Server rejected the write because someone (another tab /
			// browser / user) advanced the row past our `last_sync`. Park
			// the snapshot in the conflict map for the UI to pick up; do
			// NOT touch the local `lastSync` (the next save retries from
			// the same baseline so the conflict persists until resolved).
			conflicts.set(key, {
				serverTimestamp: resp.current_timestamp,
				localLastSync: lastSync ?? null
			})
			return
		}
		// resp.status === 'saved' — clear any prior conflict and bring
		// the local lastSync forward (or drop it entirely on a delete).
		if (opts.value === null) {
			clearLastSync(opts.workspace, opts.itemKind, opts.path)
		} else {
			setLastSync(opts.workspace, opts.itemKind, opts.path, resp.current_timestamp)
		}
		conflicts.delete(key)
		failures.delete(key)
		// Clear pending only if it's still the opts we just saved — a
		// newer `save()` that arrived during the POST replaces the entry
		// and must survive for the next flush / debouncer round.
		if (pendingSaveOpts.get(key) === opts) pendingSaveOpts.delete(key)
	} catch (e) {
		console.error('UserDraftDbSyncer.save failed', e)
		// Surface the failure to the indicator. Leave the pending opts
		// in place so the next flush / next save attempt retries the
		// same payload — clearing only on success means we don't
		// pretend the user's pending edit landed when it didn't.
		failures.set(key, true)
	}
}

/**
 * True-unload flush — `pagehide` is the browser's commitment that the
 * document is going away. Use `keepalive: true` so the network stack
 * commits the request even after the JS context is torn down; the
 * response is necessarily discarded (no listener left to read it).
 *
 * Trade-offs:
 *   - Total keepalive body size per page is capped (Chrome: 64KB). For
 *     huge editor states (low-code apps with many inline scripts) this
 *     may exceed the cap and the request will be rejected. We log and
 *     accept the loss for the oversized payload — still strictly better
 *     than the status quo, which drops every pending save.
 *   - Bypasses the debouncer/runner pipeline because both are
 *     async-scheduled and won't get a chance to run after the document
 *     hides. `debouncer.cancel(key)` is still called so a queued
 *     keystroke can't fire a second POST in between.
 *   - The keepalive POST advances `created_at` server-side but we can't
 *     read the response to update `lastSync`. That's fine on this path
 *     because the page is dying — the next mount calls
 *     `recordRemoteSync(draft_saved_at)` and reseeds from authoritative
 *     server state before any user edit can fire a save.
 */
function flushOnPageHide(): void {
	if (pendingSaveOpts.size === 0) return
	for (const [key, opts] of pendingSaveOpts) {
		debouncer.cancel(key)
		try {
			// Path encoding mirrors the generated client (`encodeURI`,
			// not `encodeURIComponent`) so slashes inside the path
			// (e.g. `u/user/myScript`) pass through.
			const url =
				OpenAPI.BASE +
				`/w/${encodeURI(opts.workspace)}` +
				`/drafts/save_draft/${encodeURI(opts.itemKind)}` +
				`/${encodeURI(opts.path)}`
			const lastSync = getLastSyncEntry(key)?.lastSync
			void fetch(url, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					value: opts.value,
					last_sync: opts.force ? undefined : lastSync,
					force: opts.force ?? false
				}),
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
	// `pagehide` is the only signal that means "the document is truly
	// going away". We deliberately do NOT listen for
	// `visibilitychange → hidden`: it fires on every tab/app switch with
	// the page surviving, and on a surviving page the debouncer's
	// pending `setTimeout` keeps running in the background, the runner
	// POST eventually fires, and the server's response updates
	// `lastSync` — no flush needed, no spurious conflict modal on the
	// user's own background-tab write.
	window.addEventListener('pagehide', flushOnPageHide)
}

/**
 * Server-side persistence for `UserDraft`. Every write is funneled
 * through the debouncer + coalescing runner above so editor autosave
 * spam can't translate into one POST per keystroke or out-of-order
 * writes under slow networks.
 *
 * Kept as a separate module so the two halves stay decoupled — `UserDraft`
 * just calls `UserDraftDbSyncer.save(...)` and doesn't reach into the
 * generated client.
 *
 * Concurrency: every save attaches the per-tab `last_sync` recorded by
 * `recordRemoteSync` (or the previous successful save). The server
 * rejects on a stale `last_sync` and `postSave` surfaces a
 * `DraftConflictInfo` for the conflict modal to resolve.
 */
export const UserDraftDbSyncer = {
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
				if (failures.has(key)) return 'failed'
				return 'none'
			}
		}
	},

	async save(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
		const key = draftKey(opts.workspace, opts.itemKind, opts.path)
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
	},

	/**
	 * Seed the per-tab `last_sync` map after an editor reads a draft
	 * from the server (via `?get_draft=true`). Calling this with the
	 * `draft_saved_at` from the response makes the next save send a
	 * matching `last_sync`, so the server accepts it unless someone
	 * pushed a newer write in between. Pass `undefined` (or omit the
	 * draftSavedAt) when no draft existed — that flips the next save
	 * back to the "no last_sync" branch, which the backend treats as a
	 * first-time push.
	 */
	recordRemoteSync(query: UserDraftLastSyncQuery, draftSavedAt: string | undefined): void {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		if (draftSavedAt) {
			setLastSync(query.workspace, query.itemKind, query.path, draftSavedAt)
		} else {
			clearLastSync(query.workspace, query.itemKind, query.path)
		}
		// Reading the server's authoritative timestamp resets the
		// conflict state and clears any prior failure flag — by
		// definition we're now back in sync with the server.
		conflicts.delete(key)
		failures.delete(key)
	},

	/**
	 * Reactive snapshot of the conflict (if any) for a draft. The
	 * returned handle reads `conflicts` via a SvelteMap getter so a
	 * `$derived` re-runs when the entry appears or clears.
	 */
	getConflict(query: UserDraftLastSyncQuery): {
		readonly conflict: DraftConflictInfo | undefined
	} {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		return {
			get conflict() {
				return conflicts.get(key)
			}
		}
	},

	/**
	 * Clear the conflict snapshot for a draft. Call after the
	 * resolution UI lands a fresh read (the editor reloaded from
	 * server) — `recordRemoteSync` does this implicitly, so the only
	 * standalone use is the "dismiss without resolving" path.
	 */
	clearConflict(query: UserDraftLastSyncQuery): void {
		conflicts.delete(draftKey(query.workspace, query.itemKind, query.path))
	},

	/**
	 * Force-save: bypass the `last_sync` check and overwrite the
	 * server row. Used by the conflict-resolution modal's "Overwrite
	 * the remote" action. Goes through the same coalescing runner
	 * (and resolves only after the POST lands) so the caller can `await`
	 * it before navigating away or refetching.
	 */
	async overwrite(opts: Omit<UserDraftDbSyncerSaveOpts, 'force'>): Promise<void> {
		await this.save({ ...opts, immediate: true, force: true })
	},

	/**
	 * Flush whatever's queued in the autosave debouncer for this draft
	 * RIGHT NOW. Use for explicit "save now" shortcuts (Ctrl/Cmd+S).
	 *
	 * Re-submits the latest opts captured by `save()` (held in
	 * `pendingSaveOpts` until the POST lands) with `immediate: true`,
	 * which cancels the queued debouncer and routes through the
	 * coalescing runner. The returned promise resolves only after the
	 * POST lands, so callers can `await flush(...); show "Saved"`.
	 *
	 * No-op when nothing is pending (the most recent save already
	 * landed, or the editor never autosaved this entry). The caller
	 * should NOT assume "no pending" means "nothing to save" — Monaco
	 * may still have unmaterialized text in its own buffer; flush the
	 * editor first (`Editor.flushPendingChanges()`) and await `tick()`
	 * so the bind:code propagation reaches our save() before this.
	 */
	async flush(query: UserDraftLastSyncQuery): Promise<void> {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		const opts = pendingSaveOpts.get(key)
		if (!opts) return
		await this.save({ ...opts, immediate: true })
	}
}
