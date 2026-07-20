import { SvelteMap } from 'svelte/reactivity'
import { DraftService, type UserDraftItemKind } from './gen'
import { OpenAPI } from './gen/core/OpenAPI'
import { createCoalescingKeyedRunner, CoalescingDisplacedError } from './coalescingRunner.svelte'
import { createDebouncerByKey } from './debouncerByKey.svelte'
import { setLocalDraftHint } from './localDraftHints.svelte'

/**
 * Per-draft baseline timestamp attached as `last_sync` on the next save so
 * the backend can reject stale writes (`created_at <= last_sync`).
 *
 * MUST be tab-local, not localStorage: each tab tracks its own baseline.
 * Sharing it would let tab-1's save advance tab-2's `last_sync` to a fresh
 * timestamp, so tab-2's next save would pass the server's WHERE check and
 * clobber tab-1 — the exact conflict this feature prevents. The cost: a
 * fresh tab has no `last_sync` (first save lands unconditionally), but the
 * editor's `recordRemoteSync` reseeds from the server timestamp on load
 * before any user edit can save.
 */
type DraftLastSyncEntry = { lastSync: string }
const lastSyncMap = new Map<string, DraftLastSyncEntry>()

/** Must match `mapKey` in `userDraft.svelte.ts`. */
function draftKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

/**
 * Top-level script fields stripped before a draft is persisted. `hash` is the
 * deployed version's identity (server-managed, re-supplied from the deployed row
 * on load) and `assets` is re-derived from the script content by the editor —
 * neither is draft content, so saving them bloats the row and resurfaces as
 * fork/workspace diff noise. The editing object carries them because
 * `getScriptByPath` returns the full DB row (since #9351).
 *
 * `CLEANED_VALUE_KEYS` (frontend/src/lib/utils.ts) strips a SUPERSET of these
 * from the diff view (it also drops `inherited_labels` and other bookkeeping
 * keys). The two lists are deliberately NOT the same: the diff hides every
 * non-content key, while this sanitizer only trims the two that meaningfully
 * bloat the persisted draft. Just keep the hash/assets entries here in step
 * with that set; the rest may diverge.
 */
const SCRIPT_DRAFT_OMITTED_FIELDS = ['hash', 'assets'] as const

/**
 * Drop server-managed / re-derived fields a draft must not persist. Runs at the
 * single persistence chokepoint (`save`) so every path — reactive autosave,
 * Ctrl/Cmd+S flush, the `pagehide` keepalive — sends the same trimmed payload.
 * Only scripts carry these fields; other kinds pass through untouched.
 */
export function sanitizeDraftValueForSave(
	itemKind: UserDraftItemKind,
	value: unknown | null
): unknown | null {
	if (
		itemKind !== 'script' ||
		value === null ||
		typeof value !== 'object' ||
		Array.isArray(value)
	) {
		return value
	}
	const obj = value as Record<string, unknown>
	if (!SCRIPT_DRAFT_OMITTED_FIELDS.some((f) => f in obj)) return value
	const clone = { ...obj }
	for (const f of SCRIPT_DRAFT_OMITTED_FIELDS) delete clone[f]
	return clone
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
	/** `null` signals a delete — same conflict rules as an upsert. */
	value: unknown | null
	/** Bypass the debouncer: cancel any pending autosave for this key (it
	 * would otherwise overwrite what we send), route through the coalescing
	 * runner to preserve ordering against an in-flight POST, and resolve only
	 * once the key's save chain has drained. Use for
	 * `await save(...); read-the-server` flows where a fire-and-forget save
	 * would race the next read.
	 *
	 * Resolving means "the key is settled", NOT "your payload won": a newer
	 * save can displace this one (it then carries the later state), and — as
	 * with every other `save` — `postSave` routes a rejected or failed POST to
	 * `conflicts` / `failures` rather than throwing. Read those to know what
	 * actually landed. */
	immediate?: boolean
	/** Skip the optimistic-concurrency check and overwrite the server row.
	 * Used by the conflict-resolution UI ("Overwrite the remote"). Default
	 * `false`: autosaves attach `last_sync` and respect a reject. */
	force?: boolean
	/** Save came from the reactive autosave mirror, not an explicit user
	 * action (Ctrl/Cmd+S flush, discard, fork, overwrite). */
	auto?: boolean
	/** Source handle opts into the "Enable auto-save" toggle. Only the
	 * full-page editors (script / flow / app / raw app) set it; their
	 * `auto` saves are suppressed while the toggle is off (latest opts
	 * still park in `pendingSaveOpts` for an explicit flush). Drawer
	 * editors (variables / resources / triggers) leave it unset and always
	 * sync. Explicit saves always go through. */
	canBeDisabled?: boolean
}

/**
 * Snapshot of a rejected save. `localLastSync` is what we sent (or `null`
 * if never synced); `serverTimestamp` is the row's current `created_at`,
 * surfaced so the resolution UI can show how recent the conflict was.
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
 * Autosave lifecycle for a single draft:
 *   - `saving`:  a POST is in flight (coalescing runner busy).
 *   - `pending`: a change is queued in the debouncer, not yet fired.
 *   - `failed`:  the last POST threw (network / 5xx) and no later attempt
 *                succeeded. Conflicts go through the modal, not here.
 *   - `none`:    in sync (or nothing happened).
 * Render priority `saving` > `pending` > `failed` > `none`: an active
 * retry outranks the prior failure so the indicator shows it in-flight.
 */
export type UserDraftSyncState = 'none' | 'pending' | 'saving' | 'failed'

export type UserDraftStateHandle = {
	/** Reactive: re-runs as the draft moves through the pipeline. */
	readonly state: UserDraftSyncState
	/** When `state === 'failed'`, the message from the thrown error.
	 * `undefined` otherwise. */
	readonly failureMessage: string | undefined
	/** Reactive: bumped each time `flush()` completes, INCLUDING the no-op
	 * path. Lets the indicator flash "Saved" on Ctrl/Cmd+S even when the
	 * autosave already landed (otherwise the shortcut is silent then). */
	readonly flushCount: number
}

/**
 * Two-stage pipeline per draft key. The debouncer collapses keystroke
 * bursts (1500ms reset, 10000ms ceiling so sustained typing still flushes
 * within 10s). When it fires, `opts` goes to the coalescing runner, which
 * keeps at most one POST in flight per key plus one "latest" follow-up —
 * newer submissions replace any prior pending, so the server never sees
 * stale-then-fresh out of order.
 *
 * Imperative awaits (delete-then-refetch) MUST NOT rely on `save()`'s
 * promise: it resolves when the work is queued, not when the POST lands.
 * Use the `immediate` bypass for those.
 */
const debouncer = createDebouncerByKey({ debounceMs: 1500, maxDebounceMs: 10000 })
const runner = createCoalescingKeyedRunner()

/**
 * "Enable auto-save" preference (AutosaveIndicator popover toggle).
 * Browser-wide and persisted. While off, `auto: true` saves and the
 * unload keepalive flush are suppressed — nothing leaves the tab except
 * explicit saves (latest opts still park in `pendingSaveOpts` for flush).
 */
const AUTOSAVE_ENABLED_LS_KEY = 'userDraftAutosaveEnabled'
function readAutosaveEnabled(): boolean {
	try {
		return localStorage.getItem(AUTOSAVE_ENABLED_LS_KEY) !== 'false'
	} catch {
		return true
	}
}
let autosaveEnabledState = $state(readAutosaveEnabled())

/**
 * Latest unconfirmed `save` opts per draft key. The unload flush fires a
 * `keepalive` POST for each entry. Cleared by `postSave` on success, but
 * only when the entry is still the same object the success was for — a
 * newer `save()` during the in-flight POST must survive for the next round.
 */
const pendingSaveOpts = new Map<string, UserDraftDbSyncerSaveOpts>()

/**
 * Keys whose saves are HARD-blocked: while editing another user's loaded draft
 * the foreign value must never reach the server through ANY path (reactive
 * mirror, explicit flush, the pagehide keepalive flush). The value is the
 * "blocked save attempted" callback — the first such attempt is the user's
 * first edit, which the overlay UI turns into an "overwrite?" prompt.
 */
const syncLocked = new Map<string, (() => void) | undefined>()

/**
 * Conflict snapshots, populated when the server rejects a save (row
 * `created_at` newer than our `last_sync`). Read via `getConflict(query)`
 * to drive the resolution modal.
 */
const conflicts = new SvelteMap<string, DraftConflictInfo>()

/**
 * Draft keys whose last save threw (network / 5xx) → extracted error
 * message. Cleared on the next success. Drives the AutosaveIndicator's
 * "Save failed" label so a silent failure can't masquerade as "Saved".
 * Conflicts are tracked separately and don't populate this map.
 */
const failures = new SvelteMap<string, string>()

/**
 * Reactive per-key counter bumped every time `flush()` completes — even
 * when there was nothing to flush. See `UserDraftStateHandle.flushCount`.
 */
const flushes = new SvelteMap<string, number>()

/**
 * Per-key listeners fired when a save for that key LANDS on the server
 * (`status === 'saved'` with a non-null value — the draft now exists
 * server-side). Distinct from `save()` resolving, which only means the work
 * was queued. Used to defer the `?new_draft` URL strip until the first
 * autosave is confirmed (see `stripNewDraftFlagOnSave`).
 */
const saveListeners = new Map<string, Set<() => void>>()

/**
 * Global listeners fired whenever ANY draft write lands on the server —
 * upserts and deletes alike. This is the invalidation hook for caches keyed
 * on persisted draft state (the chat diff snapshot): the moment a save
 * commits, the affected item can be marked stale without polling.
 */
type DraftSavedEvent = { workspace: string; itemKind: UserDraftItemKind; path: string }
const anySavedListeners = new Set<(event: DraftSavedEvent) => void>()

/**
 * Best-effort error → readable string. The generated client wraps HTTP
 * failures as `ApiError` (`body` / `statusText`); raw fetch errors are a
 * plain `Error`. Falls back to `String(e)` to avoid `[object Object]`.
 */
function formatSaveError(e: unknown): string {
	if (e == null) return 'Unknown error'
	if (typeof e === 'string') return e
	const obj = e as Record<string, any>
	const body = obj.body
	if (typeof body === 'string' && body) return body
	if (body && typeof body === 'object') {
		const inner = body.error?.message ?? body.message ?? body.error
		if (typeof inner === 'string' && inner) return inner
	}
	if (typeof obj.message === 'string' && obj.message) return obj.message
	if (typeof obj.statusText === 'string' && obj.statusText) return obj.statusText
	return String(e)
}

async function postSave(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
	const key = draftKey(opts.workspace, opts.itemKind, opts.path)
	const lastSync = getLastSyncEntry(key)?.lastSync
	try {
		const resp = await DraftService.updateDraft({
			workspace: opts.workspace,
			kind: opts.itemKind,
			path: opts.path,
			requestBody: {
				value: opts.value as any,
				// Force-saves skip the conflict check; autosaves attach
				// `last_sync` so the server can reject stale writes. Omitting
				// `last_sync` (first-ever save) hits the backend's "treat as
				// fresh" branch.
				last_sync: opts.force ? undefined : lastSync,
				force: opts.force ?? false
			}
		})
		if (resp.status === 'conflict') {
			// Someone advanced the row past our `last_sync`. Park the
			// snapshot for the UI; do NOT touch `lastSync` — the next save
			// retries from the same baseline so the conflict persists until
			// resolved.
			conflicts.set(key, {
				serverTimestamp: resp.current_timestamp,
				localLastSync: lastSync ?? null
			})
			return
		}
		// resp.status === 'saved' — advance lastSync (or drop on delete).
		if (opts.value === null) {
			clearLastSync(opts.workspace, opts.itemKind, opts.path)
		} else {
			setLastSync(opts.workspace, opts.itemKind, opts.path, resp.current_timestamp)
		}
		// postSave is the only place a draft's server-side existence changes,
		// so it's the single source for the list pages' `*` hint
		// (value !== null → exists). Every delete path clears the hint for
		// free instead of maintaining a separate source of truth.
		setLocalDraftHint(opts.workspace, opts.itemKind, opts.path, opts.value !== null)
		conflicts.delete(key)
		failures.delete(key)
		// Clear pending only if it's still the opts we just saved — a
		// newer `save()` that arrived during the POST replaces the entry
		// and must survive for the next flush / debouncer round.
		if (pendingSaveOpts.get(key) === opts) pendingSaveOpts.delete(key)
		// Notify `onSaved` subscribers only for a real persisted draft, never a
		// delete: stripping `?new_draft` after a `value: null` save would point a
		// refresh at a row that no longer exists.
		if (opts.value !== null) {
			const listeners = saveListeners.get(key)
			if (listeners) for (const l of [...listeners]) l()
		}
		// Global subscribers hear deletes too — a removed row invalidates
		// cached state the same way an upsert does.
		for (const l of [...anySavedListeners]) {
			l({ workspace: opts.workspace, itemKind: opts.itemKind, path: opts.path })
		}
	} catch (e) {
		console.error('UserDraftDbSyncer.save failed', e)
		// Leave pending opts in place so the next attempt retries the same
		// payload — we don't pretend the edit landed when it didn't.
		failures.set(key, formatSaveError(e))
	}
}

/** Keys whose `lastSync` was made stale by a `pagehide` keepalive flush
 *  (the POST advances `created_at` but its response is unreadable).
 *  Consumed by the `pageshow` handler: on a bfcache restore the SAME
 *  document comes back alive, and a save carrying the pre-flush
 *  `last_sync` would conflict against the user's own keepalive write. */
const staleSyncAfterHideFlush = new Set<string>()

/**
 * True-unload flush on `pagehide`. Uses `keepalive` so the request commits
 * after the JS context is torn down (response discarded). Bypasses the
 * debouncer/runner — both are async-scheduled and won't run post-hide. The
 * keepalive body is capped (~64KB in Chrome); oversized payloads are
 * rejected and lost (logged, still better than dropping every pending
 * save). The POST advances `created_at` but we can't read it to update
 * `lastSync` — fine here, the next mount reseeds via `recordRemoteSync`.
 */
function flushOnPageHide(): void {
	if (pendingSaveOpts.size === 0) return
	for (const [key, opts] of pendingSaveOpts) {
		// Editing another user's loaded draft: never flush the foreign value.
		if (syncLocked.has(key)) continue
		// Auto-save off: page-editor opts are dropped with the page;
		// drawer-kind pendings (no `canBeDisabled`) still flush.
		if (!autosaveEnabledState && opts.auto && opts.canBeDisabled) continue
		debouncer.cancel(key)
		try {
			// `encodeURI` (not `encodeURIComponent`), mirroring the
			// generated client, so slashes in the path pass through.
			const url =
				OpenAPI.BASE +
				`/w/${encodeURI(opts.workspace)}` +
				`/drafts/update/${encodeURI(opts.itemKind)}` +
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
				console.error('UserDraftDbSyncer: keepalive flush failed', e)
			})
			// POST advanced the row past `lastSync` and we can't read the
			// response — mark the key so a bfcache restore drops it.
			staleSyncAfterHideFlush.add(key)
		} catch (e) {
			console.error('UserDraftDbSyncer: keepalive flush threw', e)
		}
	}
	pendingSaveOpts.clear()
}

if (typeof document !== 'undefined') {
	// Only `pagehide` means the document is truly going away. Do NOT use
	// `visibilitychange → hidden`: it fires on every tab switch with the
	// page surviving, where the debounced POST still fires normally — a
	// flush there would spuriously conflict against the user's own write.
	window.addEventListener('pagehide', flushOnPageHide)
	// bfcache restore: drop the keepalive-stale entries so the next save
	// omits `last_sync` (first-push). Safe — the server copy it overwrites
	// is this document's own flush. Without this, that save is rejected as
	// a conflict and the modal opens against the user's own write.
	window.addEventListener('pageshow', (e: PageTransitionEvent) => {
		if (!e.persisted) return
		for (const key of staleSyncAfterHideFlush) {
			lastSyncMap.delete(key)
		}
		staleSyncAfterHideFlush.clear()
	})
}

/**
 * Server-side persistence for `UserDraft`. Every write goes through the
 * debouncer + coalescing runner above so autosave spam can't become one
 * POST per keystroke or out-of-order writes under slow networks.
 *
 * Concurrency: every save attaches the per-tab `last_sync` (from
 * `recordRemoteSync` or the prior success). The server rejects a stale
 * `last_sync` and `postSave` surfaces a `DraftConflictInfo` for the modal.
 */
export const UserDraftDbSyncer = {
	/**
	 * Reactive autosave-state handle for a draft. The key is captured at
	 * call time; if the consumer's `(workspace, itemKind, path)` can change,
	 * recompute the handle in a `$derived`.
	 */
	getState(query: UserDraftLastSyncQuery): UserDraftStateHandle {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		return {
			get state(): UserDraftSyncState {
				if (runner.isRunning(key)) return 'saving'
				if (debouncer.isPending(key)) return 'pending'
				if (failures.has(key)) return 'failed'
				return 'none'
			},
			get failureMessage(): string | undefined {
				// Suppress while saving/pending: an in-flight retry must not
				// show the stale message from the failure it's retrying.
				if (runner.isRunning(key) || debouncer.isPending(key)) return undefined
				return failures.get(key)
			},
			get flushCount(): number {
				return flushes.get(key) ?? 0
			}
		}
	},

	async save(opts: UserDraftDbSyncerSaveOpts): Promise<void> {
		// Trim non-draft fields once, here, so the parked opts the unload
		// keepalive flush replays carry the same payload as the live POST.
		opts = { ...opts, value: sanitizeDraftValueForSave(opts.itemKind, opts.value) }
		const key = draftKey(opts.workspace, opts.itemKind, opts.path)
		// Hard lock (editing another user's loaded draft): block EVERY save path
		// for this key — no parking, no POST. Notify the overlay so the first
		// blocked attempt (the user's first edit) can prompt before overwriting.
		if (syncLocked.has(key)) {
			syncLocked.get(key)?.()
			return
		}
		// Park the latest opts BEFORE the pipeline so the unload flush has
		// something to send even if the page hides before the debouncer fires.
		pendingSaveOpts.set(key, opts)
		if (opts.immediate) {
			// Drop the queued autosave — firing it after our POST would
			// re-save the pre-delete value. `submitAndWait` displaces the
			// runner's own pending task, so no `runner.cancel` needed.
			debouncer.cancel(key)
			try {
				await runner.submitAndWait(key, () => postSave(opts))
			} catch (e) {
				// Displacement is not a failure: a newer save took our slot, so
				// re-POSTing ours would undo it. Wait for the chain instead —
				// callers await this to know the key is settled, not to know
				// their own payload won.
				if (!(e instanceof CoalescingDisplacedError)) throw e
				await runner.settled(key)
			}
			return
		}
		// Auto-save off: opts stay parked (above) for an explicit flush but
		// never schedule a POST. Only for handles that opted into the toggle.
		if (opts.auto && opts.canBeDisabled && !autosaveEnabledState) return
		// Optimistically light the list-page `*` so it tracks the editor's
		// unsaved-changes banner without waiting for the debounced POST;
		// postSave reconciles to the confirmed server state.
		if (opts.value !== null) {
			setLocalDraftHint(opts.workspace, opts.itemKind, opts.path, true)
		}
		debouncer.schedule(key, () => {
			runner.submit(key, () => postSave(opts))
		})
	},

	/** "Enable auto-save" preference — see the module-level doc. Turning it
	 * back ON re-schedules every parked draft so edits made while off catch
	 * up without waiting for the next keystroke. */
	get autosaveEnabled(): boolean {
		return autosaveEnabledState
	},
	set autosaveEnabled(enabled: boolean) {
		autosaveEnabledState = enabled
		try {
			localStorage.setItem(AUTOSAVE_ENABLED_LS_KEY, String(enabled))
		} catch {}
		if (enabled) {
			for (const [key, opts] of pendingSaveOpts) {
				debouncer.schedule(key, () => {
					runner.submit(key, () => postSave(opts))
				})
			}
		}
	},

	/**
	 * Seed the per-tab `last_sync` after an editor reads a draft from the
	 * server. Pass the response's `draft_saved_at` so the next save sends a
	 * matching `last_sync`; pass `undefined` when no draft existed (next
	 * save omits `last_sync`, the backend's first-push branch).
	 */
	recordRemoteSync(query: UserDraftLastSyncQuery, draftSavedAt: string | undefined): void {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		if (draftSavedAt) {
			setLastSync(query.workspace, query.itemKind, query.path, draftSavedAt)
		} else {
			clearLastSync(query.workspace, query.itemKind, query.path)
		}
		// Back in sync with the server: clear any conflict / failure.
		conflicts.delete(key)
		failures.delete(key)
	},

	/**
	 * Hard-block every save for this key (editing another user's loaded draft).
	 * Cancels any in-flight/queued autosave and drops parked opts so a pending
	 * flush can't fire the user's own value either. `onBlockedAttempt` fires on
	 * each subsequent blocked save — the overlay uses it to detect the first
	 * edit. MUST pair with `unlockSync`.
	 */
	lockSync(query: UserDraftLastSyncQuery, onBlockedAttempt?: () => void): void {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		syncLocked.set(key, onBlockedAttempt)
		debouncer.cancel(key)
		runner.cancel(key)
		pendingSaveOpts.delete(key)
	},

	/** Release a `lockSync`; subsequent saves go through normally. */
	unlockSync(query: UserDraftLastSyncQuery): void {
		syncLocked.delete(draftKey(query.workspace, query.itemKind, query.path))
	},

	/**
	 * Subscribe to CONFIRMED, non-delete saves for a draft key — fired after
	 * the POST lands on the server, not when `save()` queues it. Returns an
	 * unsubscribe. Backs `stripNewDraftFlagOnSave`.
	 */
	onSaved(query: UserDraftLastSyncQuery, listener: () => void): () => void {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		let set = saveListeners.get(key)
		if (!set) {
			set = new Set()
			saveListeners.set(key, set)
		}
		set.add(listener)
		return () => {
			const s = saveListeners.get(key)
			if (!s) return
			s.delete(listener)
			if (s.size === 0) saveListeners.delete(key)
		}
	},

	/**
	 * Fires when any draft write lands on the server — upserts AND deletes,
	 * every workspace and key. For caches over persisted draft state that
	 * must invalidate the affected item the moment a write commits.
	 */
	onAnySaved(listener: (event: DraftSavedEvent) => void): () => void {
		anySavedListeners.add(listener)
		return () => {
			anySavedListeners.delete(listener)
		}
	},

	/** Reactive conflict snapshot (if any) for a draft. */
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
	 * Clear the conflict snapshot. `recordRemoteSync` does this implicitly
	 * on a fresh read, so the only standalone use is "dismiss without
	 * resolving".
	 */
	clearConflict(query: UserDraftLastSyncQuery): void {
		conflicts.delete(draftKey(query.workspace, query.itemKind, query.path))
	},

	/**
	 * Force-save: bypass the `last_sync` check and overwrite the server row
	 * (conflict modal's "Overwrite the remote"). Resolves once the key's save
	 * chain drains — see `immediate`; resolution means the chain settled, not
	 * that this force payload won (a later save can displace it). Callers
	 * `await` before navigating / refetching.
	 */
	async overwrite(opts: Omit<UserDraftDbSyncerSaveOpts, 'force'>): Promise<void> {
		await this.save({ ...opts, immediate: true, force: true })
	},

	/**
	 * Flush the draft's queued autosave NOW (explicit Ctrl/Cmd+S). Re-submits
	 * the parked opts with `immediate: true` and resolves once the key's save
	 * chain drains (see `immediate` — the parked payload may be displaced by a
	 * later save carrying newer state), so callers can `await flush(...); show
	 * "Saved"`.
	 *
	 * No-op when nothing is pending. "No pending" does NOT mean "nothing to
	 * save" — Monaco may hold unmaterialized text; flush the editor
	 * (`Editor.flushPendingChanges()`) and await `tick()` first so its
	 * bind:code reaches our save() before this.
	 *
	 * `honorAutosaveToggle` makes the flush respect the "Enable auto-save"
	 * preference: a toggle-aware autosave (`auto` + `canBeDisabled`) stays
	 * parked while auto-save is off, so the edit is NOT persisted. The editor's
	 * unmount uses it (leaving with auto-save off must not silently save —
	 * the UnsavedConfirmationModal warns instead); explicit Ctrl/Cmd+S omits it
	 * and always saves.
	 */
	async flush(
		query: UserDraftLastSyncQuery,
		opts?: { honorAutosaveToggle?: boolean }
	): Promise<void> {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		try {
			const parked = pendingSaveOpts.get(key)
			if (!parked) return
			if (opts?.honorAutosaveToggle && !autosaveEnabledState && parked.auto && parked.canBeDisabled)
				return
			await this.save({ ...parked, immediate: true })
		} finally {
			// Signal the indicator even on the no-op path so Ctrl/Cmd+S
			// shows "Saved" even when the autosave already landed.
			flushes.set(key, (flushes.get(key) ?? 0) + 1)
		}
	},

	/**
	 * Whether this draft has content edits parked but unsaved because auto-save
	 * is off — the signal the full-page editors' UnsavedConfirmationModal uses
	 * to warn before leaving. True only when auto-save is disabled AND a
	 * toggle-aware (`auto` + `canBeDisabled`) write carrying content is parked;
	 * a parked delete (`value: null` from deploy / discard / reset) is not
	 * unsaved content. Read imperatively (e.g. in `beforeNavigate`), not
	 * reactively.
	 */
	hasUnsavedDisabledChanges(query: UserDraftLastSyncQuery): boolean {
		if (autosaveEnabledState) return false
		const parked = pendingSaveOpts.get(draftKey(query.workspace, query.itemKind, query.path))
		return !!parked && parked.auto === true && parked.canBeDisabled === true && parked.value != null
	},

	/**
	 * Drop a draft's parked-but-unsaved autosave WITHOUT POSTing — the user
	 * chose to discard the auto-save-off edits on leave. Also cancels any
	 * queued debounce so turning auto-save back on can't resurrect them
	 * (the `autosaveEnabled` setter re-schedules every parked entry).
	 */
	dropPending(query: UserDraftLastSyncQuery): void {
		const key = draftKey(query.workspace, query.itemKind, query.path)
		pendingSaveOpts.delete(key)
		debouncer.cancel(key)
	}
}
