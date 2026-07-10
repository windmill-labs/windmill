import { get } from 'svelte/store'
import { onDestroy, untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { workspaceStore } from './stores'
import { readFieldsRecursively } from './utils'
import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import type { UserDraftItemKind } from './gen'

export type { UserDraftItemKind }

// Runtime mirror of the generated `UserDraftItemKind` union. `satisfies`
// + the exhaustiveness check below make a drift between the two a type error.
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
	'trigger_github',
	'data_pipeline'
] as const satisfies readonly UserDraftItemKind[]

// Reverse direction: every union member must appear in the array above.
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

type DraftState<V> = {
	val: V | undefined
}

type DraftEntry = {
	count: number
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	state: DraftState<unknown>
	/**
	 * Single-shot: skip the next reactive POST. Set by callers that already
	 * pushed the right thing (e.g. `discard`'s explicit `value: null` POST)
	 * before the reactive write, so the effect doesn't fire a duplicate.
	 */
	skipNextSync: boolean
	/**
	 * Sticky `skipNextSync`: while true the sync effect updates local state
	 * but never POSTs. For programmatic bootstrap mutations (e.g. seeding
	 * `initialCode`). Toggled via `stopSync` / `restartSync`.
	 */
	syncSuspended: boolean
	/**
	 * Single-shot: the next cell write is a programmatic SEED (baseline load
	 * / new-draft template), not a user edit. The effect adopts it as the
	 * baseline and skips the POST. Like `syncSuspended` but scoped to one
	 * write, so there's no suspension to forget to resume. Set via `seed`.
	 */
	seedNextWrite: boolean
	/**
	 * Tears down the entry's `$effect.root` scope; called at refcount 0.
	 * `undefined` only on the test-runtime fallback path (see `acquireEntry`).
	 */
	destroyRoot?: () => void
}

export type UserDraftEntry<V = unknown> = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	value: V | undefined
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
 * Map keys whose entry should start `syncSuspended` on acquire. Lets
 * callers `stopSync` BEFORE the editor has mounted (and called `use`).
 * Consumed by `acquireEntry`; cleared by the matching `restartSync`. */
const pendingSuspensions = new Set<string>()

/**
 * Synchronous read-through cache for values written via `save` while no live
 * editor entry exists. That branch persists through the debounced
 * `UserDraftDbSyncer` (async, fire-and-forget), so without this a same-tab
 * `save(...)` followed by `get(...)` would miss its own write: the global AI
 * chat writes a draft then immediately reads it back to return the result and
 * would otherwise throw "Could not read written draft". A live entry shadows
 * the cache (the entry is authoritative) and a release drops the key; a delete
 * (`remove`/`discard`) evicts it.
 */
const writtenCache = new Map<
	string,
	{ workspace: string; itemKind: UserDraftItemKind; path: string; val: unknown }
>()

function rememberWrite(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	val: unknown
): void {
	const mk = mapKey(workspace, itemKind, path)
	if (val === undefined) {
		writtenCache.delete(mk)
	} else {
		writtenCache.set(mk, { workspace, itemKind, path, val: snapshotDraftValue(val) })
	}
}

function resolveWorkspace(opts?: UserDraftOptions): string {
	const ws = opts?.workspace ?? get(workspaceStore)
	if (!ws) {
		throw new Error(
			'UserDraft: no workspace available (pass opts.workspace or set $workspaceStore)'
		)
	}
	return ws
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

/**
 * Top-level fields IGNORED when diffing a draft against its deployed
 * baseline. `permissioned_as` / `preserve_permissioned_as` are deploy
 * run-as directives, not draft content, and the editor round-trips them
 * asymmetrically (`preserve_…` rebuilt as `!!cfg.permissioned_as` on load
 * but `|| undefined` on build) — keeping them produces a phantom banner.
 *
 * The rest are server-managed read-time metadata that ride along on the
 * loaded deployed payload but never appear in the editor's draft content, so
 * comparing them would mask a true baseline match:
 * `draft_saved_at` (the draft's own save time), `edited_at` (deploy time),
 * `edited_by` (deploy author), `workspace_id`, `version_id` (deployed version),
 * and `is_draft` (backend presence flag).
 */
const DRAFT_COMPARE_IGNORED_FIELDS = [
	'permissioned_as',
	'preserve_permissioned_as',
	'extra_perms',
	'draft_saved_at',
	'edited_at',
	'edited_by',
	'workspace_id',
	'version_id',
	'parent_version',
	'is_draft'
] as const

/**
 * Normalize one side of a draft-vs-baseline comparison: JSON round-trip
 * (drafts are stored as json server-side, which strips `undefined` keys,
 * so `{ labels: undefined }` and `{}` must compare equal) and drop the
 * ignored fields above. Returns the input unchanged if unserializable.
 */
export function normalizeDraftForCompare<V>(value: V): V {
	try {
		const v = JSON.parse(JSON.stringify(value))
		if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
			for (const f of DRAFT_COMPARE_IGNORED_FIELDS) delete v[f]
		}
		return v as V
	} catch {
		return value
	}
}

/**
 * Normalized deep equality for draft values — THE "diverges from deployed
 * baseline" check. Editors must use it for BOTH their "unsaved changes"
 * banner and the `discardIf` predicate so the two can't disagree.
 */
export function draftValuesEqual(a: unknown, b: unknown): boolean {
	if (a === undefined || b === undefined) return a === b
	return deepEqual(normalizeDraftForCompare(a), normalizeDraftForCompare(b))
}

export type UserDraftHandle<V> = {
	get draft(): V | undefined
	set draft(value: V | undefined)
}

export const UserDraft = {
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// The reactive effect in `acquireEntry` observes this write
			// and POSTs it.
			entry.state.val = value
		} else {
			// No live handle: remember the value so a same-tab read-after-write
			// observes it synchronously (the syncer POST below is debounced),
			// then persist. The next editor mount re-fetches from the backend.
			rememberWrite(ws, itemKind, path, value)
			void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value })
		}
	},

	/**
	 * Current draft value from the in-memory cell. `undefined` when no
	 * editor has mounted a handle for this key in this tab.
	 */
	get<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions
	): V | undefined {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return snapshotDraftValue(entry.state.val as V | undefined)
		const cached = writtenCache.get(mk)
		if (cached) return snapshotDraftValue(cached.val as V | undefined)
		return undefined
	},

	/**
	 * Whether a draft exists for this key in the in-memory cache. False when
	 * no editor has mounted a handle for it yet.
	 */
	has(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): boolean {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) return entry.state.val !== undefined
		return writtenCache.get(mk)?.val !== undefined
	},

	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Clear the cell so live observers see the delete; arm
			// `skipNextSync` since we POST the `null` explicitly below.
			entry.skipNextSync = true
			entry.state.val = undefined
		}
		writtenCache.delete(mk)
		void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value: null })
	},

	clear(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		UserDraft.discard(itemKind, path, undefined, opts)
	},

	/**
	 * Suspend reactive sync for this key: writes still update the cell and
	 * subscribers but don't POST. Use to bracket programmatic bootstrap
	 * mutations (seeding `initialCode`, app init) so they don't appear as
	 * the user's "first edit".
	 *
	 * Safe BEFORE the entry is live (queued, applied on acquire). MUST pair
	 * every `stopSync` with a `restartSync` — forgetting silently disables
	 * autosave for the rest of the session.
	 */
	stopSync(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) entry.syncSuspended = true
		else pendingSuspensions.add(mk)
	},

	/**
	 * Resume reactive sync after `stopSync`. Subsequent differing writes
	 * POST normally; writes made during the suspension are dropped from the
	 * server's view (the cell still reflects them). Also clears a queued
	 * (pre-acquire) suspension.
	 */
	restartSync(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		pendingSuspensions.delete(mk)
		const entry = entries.get(mk)
		if (entry) entry.syncSuspended = false
	},

	/**
	 * Load `value` into the cell as a SEED (baseline reload / new-draft
	 * template) that must NOT POST as the user's edit. Reactive readers
	 * update immediately; the sync effect adopts it as the new baseline.
	 *
	 * Prefer over the `stopSync`/`restartSync` bracket for a single write —
	 * scoped to exactly this write, nothing to forget to resume. The bracket
	 * is still needed when a write fans out across components (e.g. an
	 * editor's `initContent` cascading into the bound value).
	 *
	 * No-op if the entry isn't live yet (acquire via `use`/`useMany` first).
	 */
	seed<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (!entry) return
		entry.seedNextWrite = true
		entry.state.val = snapshotDraftValue(value)
	},

	/**
	 * Currently-mounted live entries for `workspace` (in-tab only — for a
	 * workspace-wide view call `DraftService` directly).
	 */
	list<V = unknown>(opts?: UserDraftListOptions): UserDraftEntry<V>[] {
		const ws = resolveWorkspace(opts)
		const itemKinds = opts?.itemKinds ?? USER_DRAFT_ITEM_KINDS
		const out: UserDraftEntry<V>[] = []
		const seen = new Set<string>()
		for (const entry of entries.values()) {
			if (entry.workspace !== ws || !itemKinds.includes(entry.itemKind)) continue
			const val = untrack(() => entry.state.val as V | undefined)
			if (val === undefined) continue
			seen.add(mapKey(entry.workspace, entry.itemKind, entry.path))
			out.push({
				workspace: entry.workspace,
				itemKind: entry.itemKind,
				path: entry.path,
				value: snapshotDraftValue(val)
			})
		}
		// Drafts written without a live entry (e.g. global AI chat) live only in
		// `writtenCache`; surface them too so the list matches what `get` returns.
		for (const cached of writtenCache.values()) {
			if (cached.workspace !== ws || !itemKinds.includes(cached.itemKind)) continue
			const mk = mapKey(cached.workspace, cached.itemKind, cached.path)
			if (seen.has(mk)) continue
			out.push({
				workspace: cached.workspace,
				itemKind: cached.itemKind,
				path: cached.path,
				value: snapshotDraftValue(cached.val as V | undefined)
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
	 * Like `remove`, but also resets any live handle's `draft` to `fallback`
	 * in-memory (reactive readers see it at once). The explicit `value: null`
	 * POST is the canonical delete; the cell update is UI convenience.
	 *
	 * `fallback` MUST be deep-cloned: otherwise a caller passing their own
	 * live `$state` baseline (e.g. `initialStates[ws]`) would share a proxy
	 * with `handle.draft`, so edits mutate both sides and the dirty check
	 * never fires.
	 */
	discard<V>(
		itemKind: UserDraftItemKind,
		path: string,
		fallback: V | undefined,
		opts?: UserDraftOptions & {
			/** Mark the `value: null` POST as a reactive autosave (subject to
			 * the auto-save toggle) instead of an explicit action. Set by the
			 * trigger persist-effect's at-baseline discard; explicit discards
			 * leave it unset. */
			auto?: boolean
		}
	): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		const safeFallback = snapshotDraftValue(fallback)
		if (entry) {
			entry.skipNextSync = true
			entry.state.val = safeFallback
		}
		// The draft is deleted server-side (the `null` POST below); the fallback
		// only resets the live handle's UI. Drop the cache so a no-entry read
		// reports "no draft" rather than the discarded value.
		writtenCache.delete(mk)
		void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value: null, auto: opts?.auto })
	},

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftOptions & {
			/** See the `useMany` spec field. Default `false`. */
			canBeDisabled?: boolean
			/** See the `useMany` spec field. Captured once on first acquire. */
			discardIf?: (val: V) => boolean
		}
	): UserDraftHandle<V> {
		// Single-spec wrapper around `useMany`. `untrack` captures reactive
		// opts (e.g. `$workspaceStore`) once: the handle stays bound to this
		// workspace until unmount. For reactive `(kind, path)` use `useReactive`.
		const handles = UserDraft.useMany<V>(() =>
			untrack(() => [
				{
					itemKind,
					path,
					workspace: opts?.workspace,
					canBeDisabled: opts?.canBeDisabled,
					discardIf: opts?.discardIf
				}
			])
		)
		return handles[0]
	},

	/**
	 * Reactive single-spec variant of `use()`. `getSpec` is read inside the
	 * `useMany` reconcile, so a changed `(workspace, kind, path)` releases the
	 * old entry and acquires a new one. The returned object is a stable proxy
	 * forwarding `draft` to the current handle, so `bind:` lvalues survive
	 * re-keying. Use for reactive paths (`/scripts/edit/[...path]`).
	 */
	useReactive<V = unknown>(
		getSpec: () => {
			itemKind: UserDraftItemKind
			path: string
			workspace?: string
			canBeDisabled?: boolean
			/** See the `useMany` spec field. Seeds the cell on first acquire
			 *  without POSTing. Pass a STABLE reference (read it under `untrack`)
			 *  — it's consumed once, so re-reading reactive state here only churns
			 *  the reconcile. */
			defaultValue?: V
			/** See the `useMany` spec field. Captured per re-keyed acquire. */
			discardIf?: (val: V) => boolean
		}
	): UserDraftHandle<V> {
		const handles = UserDraft.useMany<V>(() => [getSpec()])
		return {
			get draft(): V | undefined {
				return handles[0]?.draft
			},
			set draft(value: V | undefined) {
				const h = handles[0]
				if (h) h.draft = value
			}
		}
	},

	useMany<V = unknown>(
		getSpecs: () => {
			itemKind: UserDraftItemKind
			path: string
			workspace?: string
			/**
			 * Value the entry's cell is seeded with on first acquire. Swallowed
			 * by the syncer's seed guard so it never POSTs. Ignored if the
			 * entry already exists (refcount > 0, e.g. another live handle
			 * keeps its value).
			 */
			defaultValue?: V
			/**
			 * Predicate: is the value about to autosave back at the deployed
			 * baseline? When true, the mirror POSTs a delete instead of a
			 * baseline-equal copy — keeping it would leave `is_draft` (and the
			 * list `*`) stuck on. MUST use the same comparison as the "unsaved
			 * changes" banner (`draftValuesEqual`) so the two can't disagree.
			 * Return false for draft-only items (no deployed copy — deleting on
			 * equality would destroy the item). Captured on first acquire.
			 */
			discardIf?: (val: V) => boolean
			/**
			 * Subject autosaves to the auto-save toggle. Only the full-page
			 * editors (script / flow / app / raw app) opt in; while off their
			 * keystroke saves park for Ctrl/Cmd+S. Default `false`: drawer
			 * editors always sync. Captured on first acquire.
			 */
			canBeDisabled?: boolean
		}[]
	): UserDraftHandle<V>[] {
		// Handles array reconciled against `getSpecs()`, indices aligned.
		// Same-key handles are reused across reconciles so callers can hold
		// a stable reference — only the entry's refcount moves.
		const handles = $state<UserDraftHandle<V>[]>([])
		const acquired = new Set<string>()
		const handleCache = new Map<string, UserDraftHandle<V>>()
		// `defaultValue` reference last used to seed each detached (empty-path)
		// handle. The reference is stable within an editing session but swapped
		// for a fresh clone each time the caller restarts (e.g. reopening the
		// new-item drawer) — so a change here means "re-seed", not "live edit".
		const detachedSeeds = new Map<string, unknown>()

		function reconcile() {
			const specs = getSpecs()
			const seen = new Set<string>()
			const next: UserDraftHandle<V>[] = []

			for (const spec of specs) {
				// Resolve the workspace WITHOUT throwing: a reactive caller (e.g. an
				// SDK editor mounted before login) may not have one yet. An absent
				// workspace is handled like an empty path below, so the handle
				// re-keys into a real entry once the workspace resolves.
				const ws = spec.workspace ?? get(workspaceStore) ?? undefined
				const mk = mapKey(ws ?? '', spec.itemKind, spec.path)

				// No workspace yet, or empty path = no draftable item (e.g. a
				// read-only historical-hash view that still binds an editor value).
				// Acquiring would mirror edits into an unroutable
				// `POST /drafts/update/kind/` (permanent "Save failed").
				// Hand out a detached, local-only handle instead.
				if (!ws || !spec.path) {
					seen.add(mk)
					let handle = handleCache.get(mk)
					// Drop the cached handle when the caller hands in a fresh
					// `defaultValue` reference (reopening the new-item drawer seeds a
					// new clone) so the rebuilt handle re-seeds instead of replaying
					// the previous session's edits. Stable reference within a session
					// means live edits are never clobbered.
					if (handle && detachedSeeds.get(mk) !== spec.defaultValue) {
						handleCache.delete(mk)
						handle = undefined
					}
					if (!handle) {
						// Seed with `defaultValue` so consumers (e.g. the new-variable
						// drawer, whose path is empty until the user types one) get a
						// populated cell to bind their form to instead of `undefined`.
						handle = makeDetachedHandle<V>(spec.defaultValue)
						handleCache.set(mk, handle)
						detachedSeeds.set(mk, spec.defaultValue)
					}
					next.push(handle)
					continue
				}
				seen.add(mk)

				if (!acquired.has(mk)) {
					acquireEntry(
						ws,
						spec.itemKind,
						spec.path,
						spec.defaultValue,
						spec.discardIf as ((val: unknown) => boolean) | undefined,
						spec.canBeDisabled ?? false
					)
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

			// Detached handles (empty-path) live only in `handleCache` — they're
			// never in `acquired`. Drop any that fell out of the specs so they
			// don't leak and a later reappearance rebuilds from scratch.
			for (const mk of [...handleCache.keys()]) {
				if (!acquired.has(mk) && !seen.has(mk)) {
					handleCache.delete(mk)
					detachedSeeds.delete(mk)
				}
			}

			// Skip no-op mutations (cached handles → reference-equal arrays).
			// `untrack` so this effect doesn't subscribe to its own `handles`
			// write — otherwise it self-loops (`effect_update_depth_exceeded`).
			untrack(() => {
				const unchanged = handles.length === next.length && handles.every((h, i) => h === next[i])
				if (!unchanged) handles.splice(0, handles.length, ...next)
			})
		}

		// Synchronous initial reconcile so single-spec callers get a populated
		// `handles[0]` before returning. `untrack` here — the `$effect` below
		// picks up subsequent changes.
		untrack(reconcile)
		$effect(reconcile)
		onDestroy(() => {
			// Flush each entry's pending autosave BEFORE releasing it. SPA
			// nav doesn't fire `pagehide`, so a debounced edit would silently
			// vanish when the editor unmounts mid-typing. Fire-and-forget —
			// the POST rides the runner's own lifetime, which outlives this
			// component, so destroying the cell here doesn't cancel it.
			//
			// `honorAutosaveToggle`: this unmount flush is an implicit autosave,
			// so a toggle-aware handle whose auto-save is off must NOT persist on
			// leave — the editor's UnsavedConfirmationModal warns the user instead.
			for (const mk of acquired) {
				const entry = entries.get(mk)
				if (!entry) continue
				void UserDraftDbSyncer.flush(
					{
						workspace: entry.workspace,
						itemKind: entry.itemKind,
						path: entry.path
					},
					{ honorAutosaveToggle: true }
				)
			}
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
	defaultValue?: unknown,
	discardIf?: (val: unknown) => boolean,
	canBeDisabled = false
): void {
	const mk = mapKey(workspace, itemKind, path)
	const existing = entries.get(mk)
	if (existing) {
		existing.count++
		return
	}
	// Seed the cell with `defaultValue` (deep-cloned). Swallowed by
	// `skipNextWrite` below — it never POSTs.
	const seed = defaultValue !== undefined ? snapshotDraftValue(defaultValue) : undefined
	// `$effect.root` gives the entry its own scope, disposed only by
	// `releaseEntry`. Without it the sync `$effect` would parent to
	// `useMany`'s reconcile effect and die on the next reconcile.
	let stateRef: DraftState<unknown> | undefined
	const destroyRoot = $effect.root(() => {
		const cell = $state<{ val: unknown }>({ val: seed })
		stateRef = cell as DraftState<unknown>
		// Mirror every observable change of `cell.val` to the syncer.
		// `readFieldsRecursively` walks the value so deep mutations
		// (`handle.draft.content = '...'`) re-fire the effect — reading
		// `cell.val` alone only subscribes to the proxy root.
		//
		// `lastSerialized` + `skipNextWrite` dedup no-op updates and treat
		// the FIRST change after mount as the seed/restore (no POST), so
		// landing on `?new_draft` doesn't sync until the user edits.
		//
		// `cell.val === undefined` is the delete signal (`value: null`).
		// `skipNextSync` lets callers that already POSTed (`discard`,
		// `remove`) suppress the duplicate fire from their own write.
		let lastSerialized: string | undefined = undefined
		let skipNextWrite = true
		$effect(() => {
			const val = cell.val
			if (val !== undefined) readFieldsRecursively(val)
			const next = val === undefined ? undefined : JSON.stringify(val)
			if (next === lastSerialized) {
				// No-op write. If a `seed` re-seeded the value already in the
				// cell, its one-shot flag consumed nothing — defuse it here or
				// it lingers and swallows the user's NEXT edit. (`skipNextWrite`
				// stays armed: an undefined-seeded cell's initial run lands
				// here, and page editors rely on it to swallow their load write.)
				const e = entries.get(mk)
				if (e?.seedNextWrite) e.seedNextWrite = false
				return
			}
			lastSerialized = next
			if (skipNextWrite) {
				skipNextWrite = false
				// One write consumes BOTH one-shot guards: a `seed` on a fresh
				// entry (still armed with the first-write skip) must not leave
				// `seedNextWrite` behind to swallow the user's first edit.
				const fresh = entries.get(mk)
				if (fresh?.seedNextWrite) fresh.seedNextWrite = false
				return
			}
			const entry = entries.get(mk)
			// `seed` write: baseline already advanced above; don't POST.
			if (entry?.seedNextWrite) {
				entry.seedNextWrite = false
				return
			}
			if (entry?.skipNextSync) {
				entry.skipNextSync = false
				return
			}
			// `syncSuspended` swallows the POST but `lastSerialized` advanced
			// above, so only writes made during suspension are dropped — the
			// first change after resume is still detected.
			if (entry?.syncSuspended) return
			// At the deployed baseline → sync a delete, not a baseline-equal
			// copy. `untrack` so reactive reads in the predicate (the editor's
			// post-deploy baseline) don't re-fire the mirror.
			const atBaseline = untrack(() => val !== undefined && (discardIf?.(val) ?? false))
			void UserDraftDbSyncer.save({
				workspace,
				itemKind,
				path,
				value: val === undefined || atBaseline ? null : val,
				// Reactive keystroke mirror — `auto`, so suppressed while the
				// auto-save toggle is off (for `canBeDisabled` handles).
				auto: true,
				canBeDisabled
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
			seedNextWrite: false,
			destroyRoot
		})
		return
	}
	// Fallback for the vitest runtime where `$effect.root`'s callback isn't
	// invoked (unreachable in production). No sync effect — writes in tests
	// stay in-memory.
	const fallback = $state<{ val: unknown }>({ val: seed })
	entries.set(mk, {
		count: 1,
		workspace,
		itemKind,
		path,
		state: fallback as DraftState<unknown>,
		skipNextSync: false,
		syncSuspended: pendingSuspensions.delete(mk),
		seedNextWrite: false
	})
}

function releaseEntry(mk: string): void {
	const entry = entries.get(mk)
	if (!entry) return
	entry.count--
	if (entry.count <= 0) {
		// The live entry was authoritative while mounted; once gone, drop any
		// cached write for this key so a later read falls back to the server
		// rather than a value the editor may have changed in the meantime.
		writtenCache.delete(mk)
		entry.destroyRoot?.()
		entries.delete(mk)
	}
}

/**
 * Local-only handle for empty-path specs: a reactive cell that supports
 * `bind:` but is wired to nothing (no entry, no sync, no POSTs). For views
 * that bind an editor value with no draftable item behind it.
 */
function makeDetachedHandle<V>(defaultValue?: V): UserDraftHandle<V> {
	let val = $state<V | undefined>(snapshotDraftValue(defaultValue))
	return {
		get draft(): V | undefined {
			return val
		},
		set draft(value: V | undefined) {
			val = value
		}
	}
}

function makeHandle<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): UserDraftHandle<V> {
	// Reads `entries.get(mk)` on every access; the entry is stable while
	// refcount > 0. After destruction, reads return `undefined` rather than
	// throwing — the consumer should already be torn down by then.
	const mk = mapKey(workspace, itemKind, path)
	const stateOf = (): DraftState<unknown> | undefined => entries.get(mk)?.state
	return {
		get draft(): V | undefined {
			return stateOf()?.val as V | undefined
		},
		set draft(value: V | undefined) {
			const state = stateOf()
			if (state) state.val = value
		}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
	liveEditorDrafts.clear()
	writtenCache.clear()
}
