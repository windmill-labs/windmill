import { get } from 'svelte/store'
import { onDestroy, untrack } from 'svelte'
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
}

export const UserDraft = {
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		const mk = mapKey(ws, itemKind, path)
		const entry = entries.get(mk)
		if (entry) {
			// Update the reactive cell. The DB sync rides on the reactive
			// effect in `acquireEntry`, which observes this write and
			// POSTs it to the syncer.
			entry.state.val = value
		} else {
			// No live handle: push directly to the syncer. The next time
			// an editor mounts for this (workspace, kind, path) it will
			// re-fetch the draft from the backend.
			void UserDraftDbSyncer.save({ workspace: ws, itemKind, path, value })
		}
	},

	/**
	 * Read the current draft value from the in-memory cell. Returns
	 * `undefined` when no editor has mounted a handle for this
	 * `(workspace, kind, path)` in this tab.
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
		return snapshotDraftValue(entry.state.val as V | undefined)
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
		if (entry) entry.syncSuspended = true
		else pendingSuspensions.add(mk)
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
		pendingSuspensions.delete(mk)
		const entry = entries.get(mk)
		if (entry) entry.syncSuspended = false
	},

	/**
	 * List currently-mounted live entries for `workspace`. Limited to
	 * in-tab handles — for a workspace-wide view across sessions, call
	 * `DraftService` directly.
	 */
	list<V = unknown>(opts?: UserDraftListOptions): UserDraftEntry<V>[] {
		const ws = resolveWorkspace(opts)
		const itemKinds = opts?.itemKinds ?? USER_DRAFT_ITEM_KINDS
		const out: UserDraftEntry<V>[] = []
		for (const entry of entries.values()) {
			if (entry.workspace !== ws || !itemKinds.includes(entry.itemKind)) continue
			const val = untrack(() => entry.state.val as V | undefined)
			if (val === undefined) continue
			out.push({
				workspace: entry.workspace,
				itemKind: entry.itemKind,
				path: entry.path,
				value: snapshotDraftValue(val)
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
			entry.state.val = safeFallback
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

	useMany<V = unknown>(
		getSpecs: () => {
			itemKind: UserDraftItemKind
			path: string
			workspace?: string
			/**
			 * Value the entry's cell is seeded with on first acquire. Swallowed
			 * by the syncer's seed guard so it never POSTs — the user's first
			 * real edit is the first synced write. An entry that already exists
			 * (refcount > 0, e.g. another live handle) keeps its current value;
			 * the default is ignored in that case.
			 */
			defaultValue?: V
		}[]
	): UserDraftHandle<V>[] {
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
		return
	}
	// Seed the cell with the caller's `defaultValue` (deep-cloned so the
	// cell owns its copy). The seed is treated as the first observable
	// write and swallowed by `skipNextWrite` below — it never POSTs.
	const seed = defaultValue !== undefined ? snapshotDraftValue(defaultValue) : undefined
	// `$effect.root` gives the entry its own scope, disposed only by
	// `releaseEntry`. Without that, the sync `$effect` would parent to
	// `useMany`'s reconcile effect and be torn down on the next
	// reconcile.
	let stateRef: DraftState<unknown> | undefined
	const destroyRoot = $effect.root(() => {
		const cell = $state<{ val: unknown }>({ val: seed })
		stateRef = cell as DraftState<unknown>
		// Mirror every observable change of `cell.val` to the DB
		// syncer. Reading `cell.val` alone only subscribes to the proxy
		// root, so deep mutations (`handle.draft.content = '...'`)
		// would slip past; `readFieldsRecursively` walks the value so
		// the effect re-fires on nested writes too.
		//
		// `lastSerialized` + `skipNextWrite` dedup no-op `val` updates
		// and treat the FIRST observable change after mount as the
		// seed/restore (no sync). That matches the editor's UX where
		// landing on `?new_draft` or seeding the deployed baseline
		// shouldn't fire a POST until the user actually edits.
		//
		// `cell.val === undefined` is the delete signal — the server
		// route accepts `value: null` for that. `skipNextSync` lets
		// callers that already POSTed (e.g. `discard`, `remove`)
		// suppress a duplicate fire from their own reactive write.
		let lastSerialized: string | undefined = undefined
		let skipNextWrite = true
		$effect(() => {
			const val = cell.val
			if (val !== undefined) readFieldsRecursively(val)
			const next = val === undefined ? undefined : JSON.stringify(val)
			if (next === lastSerialized) return
			lastSerialized = next
			if (skipNextWrite) {
				skipNextWrite = false
				return
			}
			const entry = entries.get(mk)
			if (entry?.skipNextSync) {
				entry.skipNextSync = false
				return
			}
			// `syncSuspended` swallows the POST but still advances
			// `lastSerialized` (above) so when sync resumes the next
			// real change is detected as a change — only the writes
			// made during suspension are dropped from the server's view.
			if (entry?.syncSuspended) return
			void UserDraftDbSyncer.save({
				workspace,
				itemKind,
				path,
				value: val === undefined ? null : val
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
	const fallback = $state<{ val: unknown }>({ val: seed })
	entries.set(mk, {
		count: 1,
		workspace,
		itemKind,
		path,
		state: fallback as DraftState<unknown>,
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
}
