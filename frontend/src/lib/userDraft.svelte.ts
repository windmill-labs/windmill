import { get } from 'svelte/store'
import { onDestroy, untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { workspaceStore } from './stores'
import { readFieldsRecursively } from './utils'
import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from './gen'

export type { UserDraftItemKind }

// Runtime array mirroring the generated `UserDraftItemKind` union. The
// `satisfies` clause makes the compiler reject this file if the two
// drift — adding a kind to the OpenAPI schema without listing it here
// (or vice-versa) is a type error.
export const USER_DRAFT_ITEM_KINDS = [
	'script',
	'flow',
	'app',
	'raw_app',
	'resource',
	'variable',
	'trigger_schedule',
	'trigger_webhook',
	'trigger_default_email',
	'trigger_email',
	'trigger_http',
	'trigger_websocket',
	'trigger_postgres',
	'trigger_kafka',
	'trigger_nats',
	'trigger_mqtt',
	'trigger_sqs',
	'trigger_gcp',
	'trigger_azure',
	'trigger_poll',
	'trigger_cli',
	'trigger_nextcloud',
	'trigger_google',
	'trigger_github'
] as const satisfies readonly UserDraftItemKind[]

// And the reverse direction: every member of the generated union must
// appear in `USER_DRAFT_ITEM_KINDS`.
type _AssertKindsExhaustive =
	Exclude<UserDraftItemKind, (typeof USER_DRAFT_ITEM_KINDS)[number]> extends never ? true : never
const _: _AssertKindsExhaustive = true
void _

export type UserDraftOptions = {
	workspace?: string
}

export type UserDraftListOptions = UserDraftOptions & {
	itemKinds?: readonly UserDraftItemKind[]
}

/**
 * A single (kind, path, workspace) tuple that `useMany` should hold a
 * handle for. The shape mirrors `use()`'s arguments, just bundled into
 * one object so a getter can return a list of them.
 */
export type UserDraftSpec<V = unknown> = {
	itemKind: UserDraftItemKind
	path: string
	workspace?: string
	/**
	 * Value the handle reports when the entry is first acquired and no
	 * autosave is persisted. Seeded into the in-memory cell on acquire and
	 * swallowed by the sync effect so it never POSTs — the user's first
	 * real edit is the first synced write. An entry that already exists
	 * (refcount > 0, e.g. another live handle) keeps its current value; the
	 * default is ignored in that case (an existing autosave always wins).
	 */
	defaultValue?: V
}

/**
 * Snapshot of the remote item's freshness at the moment the local draft
 * was seeded. Used by editor routes to detect that the remote has moved
 * on since the user last saw it.
 *
 * - `remoteRev`: the deployed version's id/hash/timestamp at draft load.
 * - `remoteDraftRev`: the DB-draft `created_at` at draft load, only set
 *   for kinds that have a DB-draft (`script`, `flow`, `app`, `raw_app`).
 *
 * Meta is in-memory only. It's seeded by the editor on every load (from
 * the backend response) and lost on page reload — which is fine because
 * the editor reseeds it.
 */
export type UserDraftMeta = {
	remoteRev?: string | number
	remoteDraftRev?: string | number
}

/**
 * In-memory cell shape. The value + meta are wrapped together so a single
 * reactive assignment carries both, which keeps the DB sync effect's
 * "did anything change" comparison stable.
 */
type StoredDraft<V> = { value: V } & UserDraftMeta

type DraftState<V> = {
	val: StoredDraft<V> | undefined
}

type DraftEntry = {
	count: number
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	state: DraftState<unknown>
	/**
	 * Single-shot flag consumed by the reactive sync effect in
	 * `acquireEntry`. Callers that already pushed the right thing to the
	 * server (e.g. `discard` → explicit `value: null` POST) set this
	 * before the reactive write so the effect doesn't fire a second,
	 * incorrect POST.
	 */
	skipNextSync: boolean
	/**
	 * Sticky version of `skipNextSync`. While true, the reactive sync
	 * effect updates the local state but never POSTs — used by callers
	 * that programmatically mutate the draft as part of bootstrapping
	 * (e.g. setting the editor's `initialCode` after mount) and don't
	 * want those writes to land on the server as the user's "first
	 * autosave". Toggled via `UserDraft.stopSync` / `restartSync`.
	 */
	syncSuspended: boolean
	/**
	 * Tears down the `$effect.root` scope that owns the entry's sync
	 * effect. Called when the refcount hits 0. `undefined` only when the
	 * test runtime's broken `$effect.root` forced us through the
	 * fallback path (see `acquireEntry`).
	 */
	destroyRoot?: () => void
}

export type UserDraftEntry<V = unknown> = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	value: V | undefined
	meta: UserDraftMeta
}

export type LiveEditorDraft = {
	workspace: string
	itemKind: UserDraftItemKind
	storagePath: string
	effectivePath?: string
}

export type LiveEditorDraftSpec = {
	itemKind: UserDraftItemKind
	storagePath: string
	effectivePath?: string
	workspace?: string
}

export type ClearLiveEditorDraftOptions = UserDraftOptions & {
	storagePath?: string
}

const entries = new Map<string, DraftEntry>()
const liveEditorDrafts = new Map<string, LiveEditorDraft>()
/**
 * Map keys (`workspace|kind|path`) that should start `syncSuspended`
 * when their entry is acquired. Lets callers `stopSync` BEFORE an
 * editor has mounted (and called `UserDraft.use`) — for routes that
 * suspend the bootstrap save before triggering ScriptBuilder/AppEditor
 * to mount. Consumed by `acquireEntry`; the matching `restartSync`
 * (or a second `stopSync` on the live entry) clears it normally. */
const pendingSuspensions = new Set<string>()

function resolveWorkspace(opts?: UserDraftOptions): string {
	const ws = opts?.workspace ?? get(workspaceStore)
	if (!ws) {
		throw new Error(
			'UserDraft: no workspace available (pass opts.workspace or set $workspaceStore)'
		)
	}
	return ws
}

function wrap<V>(value: V | undefined, meta?: UserDraftMeta): StoredDraft<V> | undefined {
	if (value === undefined) return undefined
	const out: StoredDraft<V> = { value }
	if (meta?.remoteRev !== undefined) out.remoteRev = meta.remoteRev
	if (meta?.remoteDraftRev !== undefined) out.remoteDraftRev = meta.remoteDraftRev
	return out
}

function unwrap<V>(stored: StoredDraft<V> | undefined): V | undefined {
	return stored?.value
}

function extractMeta(stored: StoredDraft<unknown> | undefined): UserDraftMeta {
	if (!stored) return {}
	const meta: UserDraftMeta = {}
	if (stored.remoteRev !== undefined) meta.remoteRev = stored.remoteRev
	if (stored.remoteDraftRev !== undefined) meta.remoteDraftRev = stored.remoteDraftRev
	return meta
}

/**
 * Compares the rev metadata recorded against the local draft to the current
 * backend revs. Returns the staleness cause, or `null` when the local draft
 * is still based on the latest backend state we know about.
 */
export type UserDraftStalenessCause = 'draft' | 'version'

export function checkStaleness(
	meta: UserDraftMeta,
	currentRev: string | number | undefined,
	currentDraftRev?: string | number | undefined
): UserDraftStalenessCause | null {
	if (meta.remoteRev === undefined && meta.remoteDraftRev === undefined) return null
	if (meta.remoteDraftRev !== currentDraftRev) {
		return currentDraftRev !== undefined ? 'draft' : 'version'
	}
	if (currentRev !== undefined && meta.remoteRev !== currentRev) return 'version'
	return null
}

function mapKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

function liveEditorDraftKey(workspace: string, itemKind: UserDraftItemKind): string {
	return `${workspace}/${itemKind}`
}

function snapshotDraftValue<V>(value: V | undefined): V | undefined {
	if (value === undefined) return undefined
	try {
		return structuredClone($state.snapshot(value)) as V
	} catch {
		try {
			return JSON.parse(JSON.stringify(value)) as V
		} catch {
			return undefined
		}
	}
}

export type UserDraftHandle<V> = {
	get draft(): V | undefined
	set draft(value: V | undefined)
	/**
	 * Read the rev metadata stored alongside the current draft. Empty
	 * object if the entry has no draft or no rev was ever recorded.
	 */
	get meta(): UserDraftMeta
	/**
	 * Set value AND rev metadata in one write. Later `draft = X` writes
	 * preserve the rev metadata.
	 */
	setDraftAndMeta(value: V | undefined, meta: UserDraftMeta): void
	/**
	 * Update rev metadata without touching the value. The `{ force }`
	 * option is preserved for source compatibility but is now a no-op
	 * (there is no localStorage layer to write synchronously through).
	 */
	setMeta(meta: UserDraftMeta, opts?: { force?: boolean }): void
}

/**
 * JSON round-trip normalization. Freshly-built config objects (e.g. a
 * trigger editor's `getXConfig()`) keep `undefined`-valued keys, so a
 * raw `deepEqual` reports spurious differences (`{ a: undefined }` ≠
 * `{}`). Normalize BOTH sides through the same round-trip before
 * comparing. Returns the input unchanged if it can't be serialized.
 */
export function normalizeForCompare<V>(value: V | undefined): V | undefined {
	if (value === undefined) return undefined
	try {
		return JSON.parse(JSON.stringify(value)) as V
	} catch {
		return value
	}
}

/**
 * Whether the current draft differs meaningfully from a freshly-built
 * `currentConfig`. Editor restore guards use this to decide whether to
 * overlay the draft and toast.
 *
 * Returns `false` when there is no draft. Normalizes both sides (see
 * `normalizeForCompare`) so a draft that round-trips equal to the
 * deployed config is correctly treated as "no meaningful draft".
 *
 * Typed as a guard: a `true` result narrows `localDraft` to non-nullish
 * `V`.
 */
export function localDraftDiffers<V>(
	localDraft: V | undefined | null,
	currentConfig: V
): localDraft is V {
	if (localDraft === undefined || localDraft === null) return false
	return !deepEqual(normalizeForCompare(localDraft), normalizeForCompare(currentConfig))
}

export const UserDraft = {
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Update the reactive cell — preserves any rev meta the
			// editor had seeded earlier. The DB sync rides on the
			// reactive effect in `acquireEntry`, which observes this
			// write and POSTs it to the syncer.
			const current = untrack(() => entry.state.val as StoredDraft<unknown> | undefined)
			entry.state.val = wrap(value, extractMeta(current))
		} else {
			// No live handle: push directly to the syncer. The next time
			// an editor mounts for this (workspace, kind, path) it will
			// re-fetch the draft from the backend.
			void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value })
		}
	},

	setDraftAndMeta<V>(
		itemKind: UserDraftItemKind,
		path: string,
		value: V | undefined,
		meta: UserDraftMeta,
		opts?: UserDraftOptions
	): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			entry.state.val = wrap(value, meta)
			return
		}
		// No live handle and `save_draft` requires a value — skip the
		// sync on `undefined` (delete-via-static-write), which the route
		// can't represent. Use `discard` for that path.
		if (value !== undefined) {
			void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value })
		}
	},

	/**
	 * Read the current draft value from the in-memory cell. Returns
	 * `undefined` when no editor has mounted a handle for this
	 * `(workspace, kind, path)` in this tab — UserDraft no longer
	 * persists anywhere local, so loading is the editor's job (fetch
	 * via `get_draft=true` and seed via `setDraftAndMeta`).
	 */
	get<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): V | undefined {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (!entry) return undefined
		return snapshotDraftValue(unwrap(entry.state.val as StoredDraft<V> | undefined))
	},

	/**
	 * Update the rev metadata without touching the value. No-op when no
	 * live entry exists (there's no off-cell place to record meta now).
	 */
	saveMeta(
		itemKind: UserDraftItemKind,
		path: string,
		meta: UserDraftMeta,
		opts?: UserDraftOptions
	): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (!entry) return
		const current = untrack(() => entry.state.val as StoredDraft<unknown> | undefined)
		if (current === undefined) return
		// Meta-only writes shouldn't fire a sync — the DB doesn't store
		// rev meta and an empty POST to save_draft is wasteful.
		entry.skipNextSync = true
		entry.state.val = wrap(current.value, meta)
	},

	/**
	 * Read the rev metadata for the entry. Returns an empty object if
	 * there is no live entry. Useful for staleness checks.
	 */
	getMeta(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): UserDraftMeta {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (!entry) return {}
		return extractMeta(entry.state.val as StoredDraft<unknown> | undefined)
	},

	/**
	 * Whether a draft currently exists for `(workspace, itemKind, path)`
	 * in the in-memory cache. False when no editor has mounted a handle
	 * for this entry yet.
	 */
	has(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): boolean {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (!entry) return false
		return entry.state.val !== undefined
	},

	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Drop the in-memory cell value too so any live observer
			// reflects the delete immediately. Arm `skipNextSync` so the
			// reactive effect doesn't re-fire (we're already POSTing the
			// `null` below).
			entry.skipNextSync = true
			entry.state.val = undefined
		}
		void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value: null })
	},

	clear(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		UserDraft.discard(itemKind, path, undefined, opts)
	},

	/**
	 * Suspend the reactive sync for `(workspace, itemKind, path)`.
	 * Writes after this call still update the in-memory cell and any
	 * subscribers but don't POST to the syncer. Use to bracket
	 * programmatic mutations that happen during editor bootstrap (e.g.
	 * seeding script content from `initialCode`, low-code app init)
	 * so they don't appear on the server as the user's "first edit".
	 *
	 * Safe to call BEFORE the entry is live — the suspension is queued
	 * and applied when `acquireEntry` runs. Pair every `stopSync` with
	 * a `restartSync` — forgetting to resume silently turns off
	 * autosave for the rest of the session.
	 */
	stopSync(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			entry.syncSuspended = true
			console.log('[draft-sync] stopSync (entry live)', mk)
		} else {
			pendingSuspensions.add(mk)
			console.log('[draft-sync] stopSync (queued, no entry yet)', mk)
		}
	},

	/**
	 * Resume reactive sync for `(workspace, itemKind, path)` after a
	 * `stopSync`. Subsequent writes that differ from the suspended-time
	 * state are POSTed normally; writes made during the suspension are
	 * dropped from the server's view (the local cell still reflects
	 * them). Also clears any queued (pre-acquire) suspension.
	 */
	restartSync(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const hadPending = pendingSuspensions.delete(mk)
		const entry = entries.get(mk)
		if (entry) entry.syncSuspended = false
		console.log('[draft-sync] restartSync', mk, {
			entryLive: !!entry,
			clearedPending: hadPending
		})
	},

	/**
	 * List currently-mounted live entries for `workspace`. Without the
	 * localStorage layer, "list" is meaningful only for in-tab entries —
	 * for a workspace-wide view across sessions, call
	 * `DraftService.listDrafts` instead.
	 */
	list<V = unknown>(opts?: UserDraftListOptions): UserDraftEntry<V>[] {
		const ws = resolveWorkspace(opts)
		const itemKinds = opts?.itemKinds ?? USER_DRAFT_ITEM_KINDS
		const out: UserDraftEntry<V>[] = []
		for (const entry of entries.values()) {
			if (entry.workspace !== ws || !itemKinds.includes(entry.itemKind)) continue
			const stored = untrack(() => entry.state.val as StoredDraft<V> | undefined)
			if (stored === undefined) continue
			out.push({
				workspace: entry.workspace,
				itemKind: entry.itemKind,
				path: entry.path,
				value: snapshotDraftValue(unwrap(stored)),
				meta: extractMeta(stored)
			})
		}
		return out
	},

	setLiveEditorDraft(spec: LiveEditorDraftSpec): void {
		const ws = resolveWorkspace({ workspace: spec.workspace })
		liveEditorDrafts.set(liveEditorDraftKey(ws, spec.itemKind), {
			workspace: ws,
			itemKind: spec.itemKind,
			storagePath: spec.storagePath,
			effectivePath: spec.effectivePath || undefined
		})
	},

	getLiveEditorDraft(
		itemKind: UserDraftItemKind,
		opts?: UserDraftOptions
	): LiveEditorDraft | undefined {
		const ws = resolveWorkspace(opts)
		const draft = liveEditorDrafts.get(liveEditorDraftKey(ws, itemKind))
		return draft ? { ...draft } : undefined
	},

	clearLiveEditorDraft(itemKind: UserDraftItemKind, opts?: ClearLiveEditorDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const key = liveEditorDraftKey(ws, itemKind)
		const draft = liveEditorDrafts.get(key)
		if (!draft) return
		if (opts?.storagePath !== undefined && draft.storagePath !== opts.storagePath) return
		liveEditorDrafts.delete(key)
	},

	/**
	 * Like `remove`, but also resets any live handle's `draft` to
	 * `fallback` in-memory (so reactive readers see it immediately) and
	 * suppresses the reactive sync — the explicit `value: null` POST
	 * below is the canonical delete, the cell update is just a UI
	 * convenience for the deployed-baseline.
	 *
	 * `fallback` is deep-cloned before being installed — otherwise a
	 * caller who passes their own live `$state` baseline (e.g.
	 * resource/variable editors' `initialStates[ws]`) would end up with
	 * `handle.draft` and the baseline pointing at the *same* proxy;
	 * subsequent edits would mutate both sides in lock-step and the
	 * dirty check would never fire.
	 */
	discard<V>(
		itemKind: UserDraftItemKind,
		path: string,
		fallback: V | undefined,
		opts?: UserDraftOptions
	): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		const safeFallback = snapshotDraftValue(fallback)
		if (entry) {
			entry.skipNextSync = true
			entry.state.val = wrap(safeFallback) as StoredDraft<unknown> | undefined
		}
		void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value: null })
	},

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): UserDraftHandle<V> {
		// `use()` is a single-spec wrapper around `useMany`. We untrack
		// the getter so reactive opts (e.g. `$workspaceStore`) are
		// captured once at call time — the current contract is "the
		// handle stays bound to this workspace until the component
		// unmounts." Use `useMany` directly if you want spec changes to
		// release/acquire entries as you go.
		const handles = UserDraft.useMany<V>(() =>
			untrack(() => [{ itemKind, path, workspace: opts?.workspace }])
		)
		return handles[0]
	},

	useMany<V = unknown>(getSpecs: () => UserDraftSpec<V>[]): UserDraftHandle<V>[] {
		// Reactive handles array, reconciled against the latest
		// `getSpecs()` output. Indices line up with the spec array.
		// Handles for the same `(workspace, kind, path)` tuple are
		// reused across reconciles so callers can capture a reference
		// and keep it alive — only the underlying entry's refcount
		// moves.
		const handles = $state<UserDraftHandle<V>[]>([])
		const acquired = new Set<string>()
		const handleCache = new Map<string, UserDraftHandle<V>>()

		function reconcile() {
			const specs = getSpecs()
			const seen = new Set<string>()
			const next: UserDraftHandle<V>[] = []

			for (const spec of specs) {
				const ws = spec.workspace ?? resolveWorkspace()
				const mk = mapKey(ws, spec.itemKind, spec.path)
				seen.add(mk)

				if (!acquired.has(mk)) {
					acquireEntry(ws, spec.itemKind, spec.path, spec.defaultValue)
					acquired.add(mk)
				}
				let handle = handleCache.get(mk)
				if (!handle) {
					handle = makeHandle<V>(ws, spec.itemKind, spec.path)
					handleCache.set(mk, handle)
				}
				next.push(handle)
			}

			for (const mk of [...acquired]) {
				if (!seen.has(mk)) {
					releaseEntry(mk)
					acquired.delete(mk)
					handleCache.delete(mk)
				}
			}

			// Skip no-op mutations (handles are cached by mapKey, so an
			// unchanged spec set yields reference-equal arrays).
			// `untrack` so this effect doesn't subscribe to its own
			// `handles` write — otherwise it self-loops
			// (`effect_update_depth_exceeded`).
			untrack(() => {
				const unchanged = handles.length === next.length && handles.every((h, i) => h === next[i])
				if (!unchanged) handles.splice(0, handles.length, ...next)
			})
		}

		// Synchronous initial reconcile so single-spec callers (`use()`)
		// get a populated `handles[0]` before the function returns.
		// Reactive reads inside `getSpecs()` here are intentionally not
		// tracked — the `$effect` below picks up subsequent changes.
		untrack(reconcile)
		$effect(reconcile)
		onDestroy(() => {
			for (const mk of acquired) releaseEntry(mk)
			acquired.clear()
			handleCache.clear()
		})

		return handles
	}
}

function acquireEntry(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	defaultValue?: unknown
): void {
	const mk = mapKey(workspace, itemKind, path)
	const existing = entries.get(mk)
	if (existing) {
		existing.count++
		console.log('[draft-sync] acquireEntry: reuse', mk, { newCount: existing.count })
		return
	}
	console.log('[draft-sync] acquireEntry: NEW', mk, {
		hasSeed: defaultValue !== undefined,
		pendingSuspended: pendingSuspensions.has(mk)
	})
	// Seed the cell with the caller's `defaultValue` (deep-cloned so the
	// cell owns its copy and the caller's baseline can't alias it). This is
	// how editors report the deployed/draft state until the user edits —
	// the sync effect treats this first write as the seed and never POSTs
	// it (see `lastSerialized`/`skipNextWrite` below).
	const seed =
		defaultValue !== undefined
			? (wrap(snapshotDraftValue(defaultValue)) as StoredDraft<unknown> | undefined)
			: undefined
	// `$effect.root` gives the entry its own scope, disposed only by
	// `releaseEntry`. Without that, the sync `$effect` would parent to
	// `useMany`'s reconcile effect and be torn down on the next
	// reconcile.
	let stateRef: DraftState<unknown> | undefined
	const destroyRoot = $effect.root(() => {
		const cell = $state<{ val: StoredDraft<unknown> | undefined }>({ val: seed })
		stateRef = cell
		// Mirror every observable change of `cell.val` to the DB
		// syncer. Reading `cell.val` alone only subscribes to the proxy
		// root, so deep mutations (`handle.draft.content = '...'`)
		// would slip past; `readFieldsRecursively` walks the value so
		// the effect re-fires on nested writes too.
		//
		// `lastSerialized` + `skipNextWrite` mirror the dedup pattern
		// useLocalStorageValue used to have for `saveInitialValue=false`:
		// the effect ignores no-op `val` updates, and treats the FIRST
		// observable change after mount as the seed/restore (no sync).
		// That matches the editor's UX where landing on `?new_draft`
		// or seeding the deployed baseline shouldn't fire a POST until
		// the user actually edits something.
		//
		// `stored === undefined` is the delete signal — the server
		// route accepts `value: null` for that. `skipNextSync` lets
		// callers that already POSTed (e.g. `discard`, `remove`,
		// `saveMeta`) suppress a duplicate fire from their own
		// reactive write.
		// Start at `undefined` even when the cell was seeded above: that way
		// the seed is the FIRST observable change the effect sees and gets
		// swallowed by `skipNextWrite`, so seeding the deployed/draft
		// baseline never POSTs. The user's first real edit is then the first
		// synced write.
		let lastSerialized: string | undefined = undefined
		let skipNextWrite = true
		$effect(() => {
			const stored = cell.val
			if (stored !== undefined) readFieldsRecursively(stored.value)
			const next = stored === undefined ? undefined : JSON.stringify(stored)
			if (next === lastSerialized) {
				console.log('[draft-sync] effect: no change', mk, {
					nextLen: next?.length ?? 0
				})
				return
			}
			const nextLen = next?.length ?? 0
			const diff = nextLen - (lastSerialized?.length ?? 0)
			lastSerialized = next
			if (skipNextWrite) {
				skipNextWrite = false
				console.log('[draft-sync] effect: SWALLOW (skipNextWrite seed)', mk, {
					nextLen,
					diff
				})
				return
			}
			const entry = entries.get(mk)
			if (entry?.skipNextSync) {
				entry.skipNextSync = false
				console.log('[draft-sync] effect: SWALLOW (skipNextSync)', mk, { nextLen, diff })
				return
			}
			// `syncSuspended` swallows the POST but still advances
			// `lastSerialized` (above) so when sync resumes the next
			// real change is detected as a change — only the writes
			// made during suspension are dropped from the server's view.
			if (entry?.syncSuspended) {
				console.log('[draft-sync] effect: SWALLOW (syncSuspended)', mk, { nextLen, diff })
				return
			}
			console.log('[draft-sync] effect: POST', mk, {
				nextLen,
				diff,
				stack: new Error().stack?.split('\n').slice(1, 6).join('\n')
			})
			void UserDraftDbSyncer.save({
				workspace,
				itemKind,
				path,
				value: stored === undefined ? null : stored.value
			})
		})
	})
	if (stateRef) {
		entries.set(mk, {
			count: 1,
			workspace,
			itemKind,
			path,
			state: stateRef,
			skipNextSync: false,
			syncSuspended: pendingSuspensions.delete(mk),
			destroyRoot
		})
		return
	}
	// Fallback for the vitest runtime where `$effect.root`'s callback
	// isn't invoked. Unreachable in production (Svelte runs it
	// synchronously). The fallback cell has no sync effect, so writes
	// in tests stay in-memory.
	const fallback = $state<{ val: StoredDraft<unknown> | undefined }>({ val: seed })
	entries.set(mk, {
		count: 1,
		workspace,
		itemKind,
		path,
		state: fallback,
		skipNextSync: false,
		syncSuspended: pendingSuspensions.delete(mk)
	})
}

function releaseEntry(mk: string): void {
	const entry = entries.get(mk)
	if (!entry) return
	entry.count--
	if (entry.count <= 0) {
		entry.destroyRoot?.()
		entries.delete(mk)
	}
}

function makeHandle<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): UserDraftHandle<V> {
	// The handle reads `entries.get(mk)` on every access. The entry it
	// points at is stable as long as the refcount stays > 0 (which
	// `useMany` keeps the case for as long as a spec references it).
	// If the refcount drops to 0 and the entry is destroyed, reads
	// return `undefined` rather than throwing — the consumer should
	// already have been torn down by that point.
	const mk = mapKey(workspace, itemKind, path)
	const stateOf = (): DraftState<unknown> | undefined => entries.get(mk)?.state
	return {
		get draft(): V | undefined {
			return unwrap(stateOf()?.val as StoredDraft<V> | undefined)
		},
		set draft(value: V | undefined) {
			// Preserve existing rev metadata on a value edit. `untrack`
			// the read: callers often set this from inside a `$effect`
			// mirroring `$state` into the handle; a tracked read would
			// subscribe that effect to the cell it's about to write
			// (self-loop → effect_update_depth_exceeded).
			const state = stateOf()
			if (!state) return
			const current = untrack(() => state.val as StoredDraft<V> | undefined)
			state.val = wrap(value, extractMeta(current))
		},
		get meta(): UserDraftMeta {
			return extractMeta(stateOf()?.val as StoredDraft<unknown> | undefined)
		},
		setDraftAndMeta(value: V | undefined, meta: UserDraftMeta): void {
			const state = stateOf()
			if (!state) return
			state.val = wrap(value, meta)
		},
		setMeta(meta: UserDraftMeta, _opts?: { force?: boolean }): void {
			// `force` was useful when there was a localStorage layer to
			// write through synchronously. Kept in the signature for
			// source compatibility but ignored now.
			const state = stateOf()
			if (!state) return
			const current = untrack(() => state.val as StoredDraft<V> | undefined)
			if (current === undefined) return
			const entry = entries.get(mk)
			if (entry) entry.skipNextSync = true
			state.val = wrap(current.value, meta)
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
	liveEditorDrafts.clear()
}
