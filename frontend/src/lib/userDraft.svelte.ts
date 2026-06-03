import { get } from 'svelte/store'
import { onDestroy, untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import { workspaceStore } from './stores'
import { UserDraftDbService } from './userDraftDbService'

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
] as const

export type UserDraftItemKind = (typeof USER_DRAFT_ITEM_KINDS)[number]

export type UserDraftOptions = {
	workspace?: string
}

export type UserDraftUseOptions<V> = UserDraftOptions & {
	/**
	 * Initial in-memory value for this (workspace, itemKind, path). It is *not*
	 * synced to the DB — only subsequent edits are. Callers are responsible for
	 * loading the real value (e.g. from `getXByPathWithDraft`) and seeding it.
	 */
	defaultValue?: V
}

export type UserDraftListOptions = UserDraftOptions & {
	itemKinds?: readonly UserDraftItemKind[]
}

export type UserDraftSpec<V> = {
	itemKind: UserDraftItemKind
	path: string
	workspace?: string
	defaultValue?: V
}

/**
 * Vestigial type kept so callers that still pass/read a "meta" object compile.
 * The old staleness/conflict-resolution layer is gone; meta is ignored
 * (writes are dropped, reads return `{}`).
 *
 * @deprecated drafts no longer track remote revs / staleness.
 */
export type UserDraftMeta = {
	remoteRev?: string | number
	remoteDraftRev?: string | number
}

export type UserDraftEntry<V = unknown> = {
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	value: V | undefined
	/** Always `{}` — kept for shape compatibility. */
	meta: UserDraftMeta
	persisted: boolean
	live: boolean
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

/**
 * A single in-memory draft cell. Reactive via `$state` so editors can bind to
 * it; the refcount keeps it alive while at least one `use*` spec references it.
 */
class DraftCell {
	value = $state<unknown>(undefined)
}

type DraftEntry = {
	count: number
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	cell: DraftCell
}

const entries = new Map<string, DraftEntry>()
const liveEditorDrafts = new Map<string, LiveEditorDraft>()

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
	/** Set the draft and sync the change to the DB (debounced). */
	set draft(value: V | undefined)
	/**
	 * Seed the in-memory value WITHOUT syncing to the DB. Use when loading the
	 * value the editor already fetched from the backend, so re-binding it does
	 * not write it straight back.
	 */
	setInitial(value: V | undefined): void
	/** @deprecated meta is ignored — alias of `draft = value`. */
	get meta(): UserDraftMeta
	/** @deprecated meta is ignored — sets the value and syncs it. */
	setDraftAndMeta(value: V | undefined, meta?: UserDraftMeta): void
	/** @deprecated no-op (staleness tracking removed). */
	setMeta(meta?: UserDraftMeta, opts?: { force?: boolean }): void
}

/**
 * JSON round-trip normalization. Drops `undefined`-valued keys, turns `Date`
 * into a string, etc., so two structurally-equal configs compare equal even if
 * one was freshly built (keeps `undefined` keys) and the other round-tripped.
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
 * Whether `localDraft` meaningfully differs from `currentConfig` (after
 * `normalizeForCompare`). Typed as a guard so a `true` result narrows
 * `localDraft` to non-nullish `V`.
 */
export function localDraftDiffers<V>(
	localDraft: V | undefined | null,
	currentConfig: V
): localDraft is V {
	if (localDraft === undefined || localDraft === null) return false
	return !deepEqual(normalizeForCompare(localDraft), normalizeForCompare(currentConfig))
}

/**
 * Get the entry for (workspace, itemKind, path), creating a detached
 * (refcount 0) one if none exists. The `entries` map is the in-memory draft
 * store, so static `save`/`get`/`remove` must work even when no `use*` handle
 * is currently mounted. Detached entries that end up empty are swept by
 * `releaseEntry` / `setCell`.
 */
function ensureEntry(workspace: string, itemKind: UserDraftItemKind, path: string): DraftEntry {
	const mk = mapKey(workspace, itemKind, path)
	let entry = entries.get(mk)
	if (!entry) {
		entry = { count: 0, workspace, itemKind, path, cell: new DraftCell() }
		entries.set(mk, entry)
	}
	return entry
}

function setCell(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	value: unknown | undefined,
	sync: boolean
): void {
	const mk = mapKey(workspace, itemKind, path)
	const entry = ensureEntry(workspace, itemKind, path)
	entry.cell.value = value
	// Drop a detached, now-empty entry so the map doesn't grow unbounded.
	if (value === undefined && entry.count <= 0) {
		entries.delete(mk)
	}
	if (sync) {
		UserDraftDbService.save({ path, itemKind, content: snapshotDraftValue(value), workspace })
	}
}

export const UserDraft = {
	/** Set the draft value and sync it to the DB (debounced). */
	save<V>(itemKind: UserDraftItemKind, path: string, value: V, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		setCell(ws, itemKind, path, value, true)
	},

	/**
	 * @deprecated meta is ignored. Seeds the in-memory value WITHOUT syncing
	 * (it was historically used to install a known/loaded baseline). Use
	 * `save` to persist a programmatic change.
	 */
	setDraftAndMeta<V>(
		itemKind: UserDraftItemKind,
		path: string,
		value: V | undefined,
		_meta?: UserDraftMeta,
		opts?: UserDraftOptions
	): void {
		const ws = resolveWorkspace(opts)
		setCell(ws, itemKind, path, value, false)
	},

	/**
	 * Sync `value` only if it differs (after `normalizeForCompare`) from the
	 * `deployed` baseline; otherwise delete the draft.
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
		const entry = entries.get(mapKey(ws, itemKind, path))
		return snapshotDraftValue(untrack(() => entry?.cell.value as V | undefined))
	},

	/** @deprecated meta is ignored — always returns `{}`. */
	saveMeta(
		_itemKind: UserDraftItemKind,
		_path: string,
		_meta?: UserDraftMeta,
		_opts?: UserDraftOptions
	): void {},

	/** @deprecated meta is ignored — always returns `{}`. */
	getMeta(_itemKind: UserDraftItemKind, _path: string, _opts?: UserDraftOptions): UserDraftMeta {
		return {}
	},

	has(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): boolean {
		const ws = resolveWorkspace(opts)
		const entry = entries.get(mapKey(ws, itemKind, path))
		return untrack(() => entry?.cell.value) !== undefined
	},

	/** Drop the draft: clear the in-memory value and delete the DB draft. */
	remove(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		const ws = resolveWorkspace(opts)
		setCell(ws, itemKind, path, undefined, true)
	},

	clear(itemKind: UserDraftItemKind, path: string, opts?: UserDraftOptions): void {
		UserDraft.discard(itemKind, path, undefined, opts)
	},

	/**
	 * Reset the in-memory value to `fallback` (e.g. the deployed baseline) and
	 * delete the DB draft. `fallback` is deep-cloned so the handle and the
	 * caller's baseline don't share a proxy.
	 */
	discard<V>(
		itemKind: UserDraftItemKind,
		path: string,
		fallback: V | undefined,
		opts?: UserDraftOptions
	): void {
		const ws = resolveWorkspace(opts)
		const safeFallback = snapshotDraftValue(fallback)
		setCell(ws, itemKind, path, safeFallback, false)
		UserDraftDbService.save({ path, itemKind, content: undefined, workspace: ws })
	},

	list<V = unknown>(opts?: UserDraftListOptions): UserDraftEntry<V>[] {
		const ws = resolveWorkspace(opts)
		const itemKinds = opts?.itemKinds ?? USER_DRAFT_ITEM_KINDS
		const out: UserDraftEntry<V>[] = []
		for (const entry of entries.values()) {
			if (entry.workspace !== ws || !itemKinds.includes(entry.itemKind)) continue
			const value = untrack(() => entry.cell.value as V | undefined)
			if (value === undefined) continue
			out.push({
				workspace: entry.workspace,
				itemKind: entry.itemKind,
				path: entry.path,
				value: snapshotDraftValue(value),
				meta: {},
				persisted: false,
				live: true
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

	use<V = unknown>(
		itemKind: UserDraftItemKind,
		path: string,
		opts?: UserDraftUseOptions<V>
	): UserDraftHandle<V> {
		const handles = UserDraft.useMany<V>(() =>
			untrack(() => [
				{ itemKind, path, workspace: opts?.workspace, defaultValue: opts?.defaultValue }
			])
		)
		return handles[0]
	},

	useMany<V = unknown>(getSpecs: () => UserDraftSpec<V>[]): UserDraftHandle<V>[] {
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

			untrack(() => {
				const unchanged = handles.length === next.length && handles.every((h, i) => h === next[i])
				if (!unchanged) handles.splice(0, handles.length, ...next)
			})
		}

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
	const entry = ensureEntry(workspace, itemKind, path)
	entry.count++
	// A pre-existing detached entry keeps its draft; only seed the default into
	// an empty cell.
	if (entry.cell.value === undefined && defaultValue !== undefined) {
		entry.cell.value = defaultValue
	}
}

function releaseEntry(mk: string): void {
	const entry = entries.get(mk)
	if (!entry) return
	entry.count--
	// Keep entries that still hold a draft (so handle-less `get`/`list` and a
	// later remount can read them); only sweep empty ones.
	if (entry.count <= 0 && entry.cell.value === undefined) {
		entries.delete(mk)
	}
}

function makeHandle<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): UserDraftHandle<V> {
	const mk = mapKey(workspace, itemKind, path)
	const cellOf = (): DraftCell | undefined => entries.get(mk)?.cell
	return {
		get draft(): V | undefined {
			return cellOf()?.value as V | undefined
		},
		set draft(value: V | undefined) {
			const cell = cellOf()
			if (!cell) return
			const prev = untrack(() => cell.value)
			cell.value = value
			// Skip syncing no-op writes (e.g. a `bind:` writing the seeded value
			// straight back). Only real edits hit the DB.
			if (!deepEqual(normalizeForCompare(value), normalizeForCompare(prev as V | undefined))) {
				UserDraftDbService.save({ path, itemKind, content: snapshotDraftValue(value), workspace })
			}
		},
		setInitial(value: V | undefined): void {
			const cell = cellOf()
			if (!cell) return
			cell.value = value
		},
		get meta(): UserDraftMeta {
			return {}
		},
		setDraftAndMeta(value: V | undefined, _meta?: UserDraftMeta): void {
			// Seed without syncing (historically used to install a loaded baseline).
			this.setInitial(value)
		},
		setMeta(_meta?: UserDraftMeta, _opts?: { force?: boolean }): void {}
	}
}

/** Test-only: clear all in-memory entries. */
export function __resetUserDraftForTesting(): void {
	entries.clear()
	liveEditorDrafts.clear()
}
