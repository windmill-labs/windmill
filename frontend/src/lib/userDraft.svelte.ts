import { get } from 'svelte/store'
import { onDestroy, untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { workspaceStore } from './stores'
import { useLocalStorageValue } from './svelte5Utils.svelte'

export type UserDraftItemKind =
	| 'script'
	| 'flow'
	| 'app'
	| 'raw_app'
	| 'resource'
	| 'variable'
	| 'trigger_schedule'
	| 'trigger_webhook'
	| 'trigger_default_email'
	| 'trigger_email'
	| 'trigger_http'
	| 'trigger_websocket'
	| 'trigger_postgres'
	| 'trigger_kafka'
	| 'trigger_nats'
	| 'trigger_mqtt'
	| 'trigger_sqs'
	| 'trigger_gcp'
	| 'trigger_azure'
	| 'trigger_poll'
	| 'trigger_cli'
	| 'trigger_nextcloud'
	| 'trigger_google'
	| 'trigger_github'

export type UserDraftOptions = {
	workspace?: string
}

export type UserDraftUseOptions<V> = UserDraftOptions & {
	/**
	 * Initial value used when localStorage holds no draft for this
	 * (workspace, itemKind, path). It is *not* eagerly persisted — the first
	 * actual mutation is what writes to localStorage.
	 */
	defaultValue?: V
}

/**
 * A single (kind, path, workspace) tuple that `useMany` should hold a handle
 * for. The shape mirrors `use()`'s arguments, just bundled into one object
 * so a getter can return a list of them.
 */
export type UserDraftSpec<V> = {
	itemKind: UserDraftItemKind
	path: string
	workspace?: string
	defaultValue?: V
}

/**
 * Snapshot of the remote item's freshness at the moment the local draft was
 * written. Used by editor routes to detect that the remote has moved on
 * (someone else deployed, or saved a DB draft) so we can warn the user
 * before they push stale changes.
 *
 * - `remoteRev`: the deployed version's id/hash/timestamp at draft creation.
 * - `remoteDraftRev`: the DB-draft `created_at` at draft creation, only set
 *   for kinds that have a DB-draft (`script`, `flow`, `app`, `raw_app`).
 */
export type UserDraftMeta = {
	remoteRev?: string | number
	remoteDraftRev?: string | number
}

/**
 * The shape of what we actually persist. Wrapping the value lets us add
 * metadata (timestamps, originating user, schema version, ...) later
 * without breaking existing entries.
 *
 * `lastWrittenAt` is the unix-ms timestamp of the most recent write
 * (setter call or deep mutation flush). It's the GC signal —
 * `gcUserDrafts` sweeps entries that haven't been touched in N days.
 * Set at every persist via `useLocalStorageValue`'s `transformBeforePersist`,
 * `UserDraft.save`'s direct-write fallback, and `persistDirect`. Missing
 * (undefined) on entries written before this field was introduced;
 * `gcUserDrafts` backfills them on first sighting.
 */
type StoredDraft<V> = { value: V; lastWrittenAt?: number } & UserDraftMeta

function stamp<V>(stored: StoredDraft<V> | undefined): StoredDraft<V> | undefined {
	if (stored === undefined) return undefined
	return { ...stored, lastWrittenAt: Date.now() }
}

type DraftState<V> = {
	val: StoredDraft<V> | undefined
	skipNextWriteOnce(): void
}

type DraftEntry = {
	count: number
	state: DraftState<unknown>
	/**
	 * Tears down the `$effect.root` scope that owns the entry's
	 * `useLocalStorageValue` reactivity — its `$state` cell and the persist
	 * `$effect` deep-mutation loop. Called when the refcount hits 0.
	 *
	 * `undefined` only when the test runtime's broken `$effect.root` forced
	 * us through the fallback path (see `acquireEntry`).
	 */
	destroyRoot?: () => void
}

const entries = new Map<string, DraftEntry>()

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
 *
 * - Entries with no recorded meta (legacy entries written before this field
 *   existed) report `null` — we can't tell if they're stale, and we'd rather
 *   trust the local autosave than spam the user with false positives.
 * - DB-draft staleness wins over deployed-version staleness: a remote DB
 *   draft is the more recent state to reconcile against.
 * - If a DB draft existed when the local autosave was created but now no
 *   longer exists on the remote (someone discarded it), we report `version`
 *   because the deployed version is now the canonical "latest saved".
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

/**
 * Synchronously write to localStorage, bypassing the entry's setter. Used by
 * `setMeta({ force: true })` so a "Keep current draft" modal acknowledgement
 * persists even when it happens to be the first state mutation of the entry
 * (which `useLocalStorageValue`'s `saveInitialValue: false` contract would
 * otherwise skip).
 */
function persistDirect<V>(key: string, value: V | undefined, meta: UserDraftMeta): void {
	try {
		const next = stamp(wrap(value, meta))
		if (next === undefined) {
			localStorage.removeItem(key)
		} else {
			localStorage.setItem(key, JSON.stringify(next))
		}
	} catch (e) {
		console.error('UserDraft: localStorage write failed', e)
	}
}

function readPersisted<V>(key: string): StoredDraft<V> | undefined {
	try {
		const raw = localStorage.getItem(key)
		if (raw == null || raw === 'undefined') return undefined
		const parsed = JSON.parse(raw)
		// Defensive: drop entries written before the wrapping migration. Their
		// raw payload doesn't have a `.value` and would surface as undefined
		// anyway — we just don't want to confuse `has()` callers.
		if (parsed == null || typeof parsed !== 'object' || !('value' in parsed)) return undefined
		return parsed as StoredDraft<V>
	} catch (e) {
		console.error('UserDraft: localStorage read failed', e)
		return undefined
	}
}

function mapKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `${workspace}/${itemKind}/${path}`
}

function localStorageKey(workspace: string, itemKind: UserDraftItemKind, path: string): string {
	return `userdraft/w/${workspace}/${itemKind}/${path}`
}

export type UserDraftHandle<V> = {
	get draft(): V | undefined
	set draft(value: V | undefined)
	/**
	 * Read the rev metadata stored alongside the current draft. Empty object
	 * if the entry has no draft or no rev was ever recorded.
	 */
	get meta(): UserDraftMeta
	/**
	 * Atomically set the draft value AND its rev metadata in a single write.
	 *
	 * Used by editor routes to record the backend rev at load time without
	 * triggering an extra persist (combined with the value write, the
	 * underlying useLocalStorageValue's saveInitialValue=false dedup skips
	 * it). Subsequent `handle.draft = X` writes only mutate `value` and
	 * preserve whatever rev metadata is in place.
	 */
	setDraftAndMeta(value: V | undefined, meta: UserDraftMeta): void
	/**
	 * Update the rev metadata in place without touching the draft value.
	 * Used after the "Keep current draft" modal action to ack the new remote
	 * rev so the staleness modal doesn't fire again until the remote moves
	 * further.
	 *
	 * Pass `{ force: true }` to also persist the new meta to localStorage
	 * synchronously, bypassing `saveInitialValue: false`'s skip of the first
	 * state mutation. Without `force`, an ack that happens to be the first
	 * write on the entry would only update in-memory state and be lost on
	 * next mount — re-triggering the staleness modal even after the user
	 * just acknowledged it.
	 */
	setMeta(meta: UserDraftMeta, opts?: { force?: boolean }): void
}

/**
 * JSON round-trip normalization. localStorage persistence stringifies the
 * draft, which silently drops keys whose value is `undefined`, turns `Date`
 * into a string, etc. A freshly-built config object (e.g. a trigger editor's
 * `getXConfig()`) keeps those `undefined`-valued keys, so a raw
 * `deepEqual(persistedDraft, freshConfig)` reports spurious differences
 * (`{ a: undefined }` ≠ `{}`). Normalize BOTH sides through the same
 * round-trip before comparing. Returns the input unchanged if it can't be
 * serialized (e.g. a cyclic structure) — better a false "differs" than a
 * throw inside a load/effect path.
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
 * Whether the persisted local autosave (`localDraft`, as returned by
 * `UserDraft.get`) meaningfully differs from the freshly-built
 * `currentConfig`. Editor restore guards use this to decide whether to
 * overlay the local autosave and toast.
 *
 * Returns `false` when there is no local draft. Normalizes both sides (see
 * `normalizeForCompare`) so a draft that round-trips equal to the deployed
 * config — e.g. one written by merely opening then closing the editor with
 * no edits — is correctly treated as "no meaningful draft" instead of
 * spuriously triggering a restore on every reopen.
 *
 * Typed as a guard: a `true` result narrows `localDraft` to non-nullish
 * `V`, mirroring the `localCfg && …` narrowing it replaces so call sites
 * can pass the draft straight into `loadXConfig(...)` without re-checking.
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
			// Update the shared reactive state so all observers are notified.
			// The underlying useLocalStorageValue setter persists the wrapped
			// value. Preserve any existing rev metadata on the entry. The
			// meta-preservation read runs under `untrack` for the same reason
			// as `UserDraftHandle.set draft` — defensive against being called
			// from inside an effect.
			const current = untrack(() => entry.state.val as StoredDraft<unknown> | undefined)
			entry.state.val = wrap(value, extractMeta(current))
			return
		}
		// External save without a live handle: preserve any persisted meta
		// so the staleness signal isn't lost just because the editor wasn't
		// open while we wrote.
		const existing = readPersisted<unknown>(localStorageKey(ws, itemKind, path))
		try {
			localStorage.setItem(
				localStorageKey(ws, itemKind, path),
				JSON.stringify(stamp(wrap(value, extractMeta(existing))))
			)
		} catch (e) {
			console.error('UserDraft.save: localStorage write failed', e)
		}
	},

	/**
	 * Autosave gate. Persists `value` as the local draft only when it
	 * differs (after `normalizeForCompare`) from `deployed` — the backend
	 * baseline the editor loaded and keeps in sync across saves (the same
	 * value the editor's own `hasChanged` / dirty signal compares against).
	 * When equal, removes any existing draft instead.
	 *
	 * Without this, an editor's autosave `$effect` fires once as soon as it
	 * finishes loading (the `drawerLoading` flag flips and the just-loaded
	 * config counts as a "change" to the effect), so merely opening and
	 * closing an editor with no edits would leave a no-op draft behind that
	 * `has()` / restore guards then treat as unsaved work forever.
	 */
	saveIfChanged<V>(
		itemKind: UserDraftItemKind,
		path: string,
		value: V,
		deployed: V | undefined,
		opts?: UserDraftOptions
	): void {
		if (deepEqual(normalizeForCompare(value), normalizeForCompare(deployed))) {
			UserDraft.remove(itemKind, path, opts)
		} else {
			UserDraft.save(itemKind, path, value, opts)
		}
	},

	get<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): V | undefined {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			return unwrap(entry.state.val as StoredDraft<V> | undefined)
		}
		return unwrap(readPersisted<V>(localStorageKey(ws, itemKind, path)))
	},

	/**
	 * Update the rev metadata for an entry without touching the value, and
	 * persist immediately. Used by editor routes that don't hold a live
	 * handle (apps, raw apps) — they read the local draft via `UserDraft.get`
	 * and the handle is created later inside the child editor.
	 *
	 * No-op when the entry has no draft to attach meta to.
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
		if (entry) {
			const current = untrack(() => entry.state.val as StoredDraft<unknown> | undefined)
			if (current === undefined) return
			entry.state.val = wrap(current.value, meta)
		}
		const existing = readPersisted<unknown>(localStorageKey(ws, itemKind, path))
		if (existing === undefined) return
		persistDirect(localStorageKey(ws, itemKind, path), existing.value, meta)
	},

	/**
	 * Read the rev metadata for the entry. Returns an empty object if there
	 * is no entry. Useful for staleness checks before reading the draft.
	 */
	getMeta(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): UserDraftMeta {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return extractMeta(entry.state.val as StoredDraft<unknown> | undefined)
		return extractMeta(readPersisted<unknown>(localStorageKey(ws, itemKind, path)))
	},

	/**
	 * Whether a draft currently exists for (workspace, itemKind, path).
	 * Falls back to the persisted localStorage entry when no live handle is
	 * registered. Useful for distinguishing "first visit" from "returning
	 * visit with unsaved local changes".
	 */
	has(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): boolean {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return entry.state.val !== undefined
		return readPersisted(localStorageKey(ws, itemKind, path)) !== undefined
	},

	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		try {
			localStorage.removeItem(localStorageKey(ws, itemKind, path))
		} catch (e) {
			console.error('UserDraft.remove: localStorage remove failed', e)
		}
	},

	/**
	 * Discard the local autosave and reset any live handle's `draft` to
	 * `fallback`. Differs from `remove` in two ways:
	 *
	 * 1. The in-memory cell is updated, so consumers reactively reading
	 *    `handle.draft` immediately see the fallback instead of the
	 *    stale local autosave.
	 * 2. The in-memory reset is marked to skip the next persist, so the
	 *    fallback value does NOT round-trip back into localStorage. The
	 *    LS slot stays empty until the user makes a real edit.
	 *
	 * Used by route editors' "Reset to deployed" flow: pass the backend
	 * baseline as `fallback` so the form repaints against deployed state
	 * without leaving a duplicate-of-backend LS entry behind.
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
		if (entry) {
			// Arm the skip BEFORE the cell write so `useLocalStorageValue`'s
			// setter consumes it and suppresses the would-be persist. The
			// explicit `localStorage.removeItem` below is what actually
			// clears the slot (and also covers the case where no live
			// handle exists).
			entry.state.skipNextWriteOnce()
			entry.state.val = wrap(fallback) as StoredDraft<unknown> | undefined
		}
		try {
			localStorage.removeItem(localStorageKey(ws, itemKind, path))
		} catch (e) {
			console.error('UserDraft.discard: localStorage remove failed', e)
		}
	},

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftUseOptions<V>
	): UserDraftHandle<V> {
		// `use()` is a single-spec wrapper around `useMany`. We untrack the
		// getter so that reactive opts (e.g. `$workspaceStore`) are captured
		// once at call time — the current `use()` contract is "the handle
		// stays bound to this workspace until the component unmounts." Use
		// `useMany` directly if you want spec changes to release/acquire
		// entries as you go.
		const handles = UserDraft.useMany<V>(() =>
			untrack(() => [
				{
					itemKind,
					path,
					workspace: opts?.workspace,
					defaultValue: opts?.defaultValue
				}
			])
		)
		return handles[0]
	},

	useMany<V = unknown>(getSpecs: () => UserDraftSpec<V>[]): UserDraftHandle<V>[] {
		// Reactive handles array, reconciled against the latest `getSpecs()`
		// output. Indices line up with the spec array. Handles for the same
		// (workspace, kind, path) tuple are reused across reconciles so
		// callers can capture a reference and keep it alive — only the
		// underlying entry's refcount moves.
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

			// Skip the mutation when specs are structurally unchanged — handles
			// are cached by mapKey, so two reconciles with the same spec set
			// produce reference-equal arrays. Avoids dirtying downstream
			// reactive readers on no-op `$effect` re-runs.
			//
			// Both the comparison reads AND the splice's own `.length` read
			// run under `untrack` so the effect doesn't register `handles`
			// as one of its own dependencies — otherwise the splice would
			// re-fire the effect, splice again, … (Svelte raises
			// `effect_update_depth_exceeded`). The splice's downstream
			// notification still propagates; `untrack` only suppresses the
			// dependency subscription on the producer side.
			untrack(() => {
				const unchanged = handles.length === next.length && handles.every((h, i) => h === next[i])
				if (!unchanged) handles.splice(0, handles.length, ...next)
			})
		}

		// Synchronous initial reconcile so single-spec callers (`use()`) get a
		// populated `handles[0]` before the function returns. Reactive reads
		// inside `getSpecs()` here are intentionally not tracked — the
		// `$effect` below picks up any subsequent dependency changes.
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
	defaultValue: unknown
): void {
	const mk = mapKey(workspace, itemKind, path)
	const existing = entries.get(mk)
	if (existing) {
		existing.count++
		return
	}
	// `useLocalStorageValue` registers a `$state` cell and a persist `$effect`
	// for deep-mutation detection. The `$effect` is what makes
	// `handle.draft.path = 'new'` (a deep mutation, not a setter call)
	// actually hit localStorage. By default that `$effect` is parented to the
	// caller's current scope — if `useMany`'s reconcile `$effect` is the
	// caller (when a new spec is acquired during a reactive re-run), the
	// persist `$effect` becomes its child and gets torn down on the next
	// reconcile. Wrap the creation in `$effect.root` so the entry's
	// reactivity lives in its own scope and only the entry's release path
	// disposes of it.
	const useLocalStorageOptions = {
		// The first value to flow into the handle (e.g. a backend load in
		// the editor route) is the baseline — only persist when the user
		// actually changes it afterwards. Coalesce a typing storm into one
		// localStorage write per 500 ms.
		saveInitialValue: false,
		debounce: 500,
		// Stamp every write with `lastWrittenAt` so the periodic GC pass
		// can sweep entries that haven't been touched in 30 days. Done at
		// persist time (not via `wrap()`) so deep mutations — which never
		// re-run the setter — still bump the clock on each flush.
		transformBeforePersist: stamp<unknown>
	} as const
	let stateRef: DraftState<unknown> | undefined
	const destroyRoot = $effect.root(() => {
		stateRef = useLocalStorageValue<StoredDraft<unknown> | undefined>(
			localStorageKey(workspace, itemKind, path),
			wrap(defaultValue),
			undefined,
			useLocalStorageOptions
		)
	})
	if (stateRef) {
		entries.set(mk, { count: 1, state: stateRef, destroyRoot })
		return
	}
	// Fallback for the vitest runtime, where `$effect.root` returns its
	// disposer but never invokes the callback. In tests there's no outer
	// reactive effect calling acquireEntry, so the persist `$effect` parents
	// to the test scope and lives long enough for assertions. In production
	// `$effect.root` runs the callback synchronously per the Svelte 5 spec
	// and this path is unreachable.
	const state = useLocalStorageValue<StoredDraft<unknown> | undefined>(
		localStorageKey(workspace, itemKind, path),
		wrap(defaultValue),
		undefined,
		useLocalStorageOptions
	)
	entries.set(mk, { count: 1, state })
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
	// The handle reads `entries.get(mk)` on every access. The entry it points
	// at is stable as long as the refcount stays > 0 (which `useMany` keeps
	// the case for as long as a spec references it). If the refcount drops to
	// 0 and the entry is destroyed, reads return `undefined` rather than
	// throwing — the consumer should already have been torn down by that point.
	const mk = mapKey(workspace, itemKind, path)
	const stateOf = (): DraftState<unknown> | undefined => entries.get(mk)?.state
	return {
		get draft(): V | undefined {
			return unwrap(stateOf()?.val as StoredDraft<V> | undefined)
		},
		set draft(value: V | undefined) {
			// Preserve existing rev metadata when the user just edits the
			// value (e.g. typing in the editor). useLocalStorageValue's
			// setter writes synchronously and removes the localStorage
			// entry when value is undefined.
			//
			// `state.val` must be read under `untrack` — callers commonly
			// invoke this setter from inside a `$effect` that mirrors a
			// reactive `$state` into the handle. A tracked read here would
			// subscribe that effect to the entry's `$state` cell that we're
			// about to write, creating an effect_update_depth_exceeded loop.
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
		setMeta(meta: UserDraftMeta, opts?: { force?: boolean }): void {
			// Read under `untrack` for the same reason as `set draft` above —
			// avoid making any surrounding effect re-fire on the write below.
			const state = stateOf()
			if (!state) return
			const current = untrack(() => state.val as StoredDraft<V> | undefined)
			if (current === undefined) return
			state.val = wrap(current.value, meta)
			if (opts?.force) {
				persistDirect(localStorageKey(workspace, itemKind, path), current.value, meta)
			}
		}
	}
}

/**
 * Default GC retention window: 30 days. Entries that haven't been touched
 * (no setter call, no deep-mutation persist) for this long are swept on
 * the next `gcUserDrafts` invocation.
 */
export const USER_DRAFT_GC_MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000

/**
 * Sweep stale UserDraft entries from localStorage. Walks every
 * `userdraft/w/...` key, checks its `lastWrittenAt` stamp, and removes
 * any entry older than `maxAgeMs`.
 *
 * Entries written before `lastWrittenAt` was introduced lack the field;
 * we backfill them to `now()` on first sighting so they participate in
 * the next sweep cycle rather than getting wiped immediately.
 *
 * Safe to call on every load and on a timer (e.g. every 30 min) — live
 * entries get their stamp refreshed on every persist, so the sweep only
 * touches truly stale records.
 */
export function gcUserDrafts(maxAgeMs: number = USER_DRAFT_GC_MAX_AGE_MS): void {
	if (typeof localStorage === 'undefined') return
	const now = Date.now()
	const cutoff = now - maxAgeMs
	const keys: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const k = localStorage.key(i)
		if (k != null && k.startsWith('userdraft/w/')) keys.push(k)
	}
	for (const key of keys) {
		try {
			const raw = localStorage.getItem(key)
			if (raw == null) continue
			const parsed = JSON.parse(raw)
			if (parsed == null || typeof parsed !== 'object') continue
			if (typeof parsed.lastWrittenAt !== 'number') {
				// Pre-GC-feature entry. Backfill so the next sweep can decide.
				parsed.lastWrittenAt = now
				localStorage.setItem(key, JSON.stringify(parsed))
				continue
			}
			if (parsed.lastWrittenAt < cutoff) localStorage.removeItem(key)
		} catch (e) {
			console.error('UserDraft GC: failed to inspect', key, e)
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
}
