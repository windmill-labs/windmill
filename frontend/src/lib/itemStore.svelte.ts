/**
 * Per-item state for the drawer kinds (resources, variables, schedules): the value being edited,
 * the deployed value it is measured against, and the user's draft row, which is never commanded
 * — it exists exactly when `value ≠ deployed`, and follows whatever moves either side.
 *
 * Saving therefore only has to advance `deployed`: an edit typed while the write was in flight
 * still differs and keeps its row, anything else lands back on the baseline and loses it. Every
 * command runs through a per-key queue and reports an outcome, and its effects land on the entry
 * that issued it, whatever the component that asked is showing by then.
 *
 * The rules live in `Entry`, driven by explicit transitions (`touch`, the setters, the commands),
 * so they hold without an effect scheduler; in the browser a detached effect only calls `touch`
 * when a form mutates the value in place.
 */
import { onDestroy, untrack } from 'svelte'
import { deepEqual } from 'fast-equals'
import {
	draftValuesEqual,
	registerLiveItemBridge,
	type UserDraftEntry,
	type UserDraftItemKind
} from './userDraft.svelte'
import { UserDraftDbSyncer } from './userDraftDbSyncer.svelte'
import { setLocalDraftHint } from './localDraftHints.svelte'
import { readFieldsRecursively } from './utils'
import { randomUUID } from './utils/uuid'

const ITEM_KINDS = ['resource', 'variable', 'trigger_schedule'] as const
export type ItemKind = (typeof ITEM_KINDS)[number]

/**
 * - `deployed`: a server row exists; the draft row, if any, is a divergence from it.
 * - `draft`: the draft row *is* the item (`no_deployed`); discarding it removes the item.
 * - `new`: being created, under a temporary `draft:` path. Never persisted as a draft.
 */
export type ItemOrigin = 'deployed' | 'draft' | 'new'
export type ItemStatus = 'loading' | 'idle' | 'saving' | 'conflicted' | 'failed'
export type ItemKey = { workspace: string; kind: ItemKind; path: string }

export type ItemLoad<V> = {
	/** `undefined` for a draft-only item. */
	deployed?: V
	/** The user's own draft row, if any. */
	draft?: V
	draftSavedAt?: string
	/** Only for a temporary path: what the create starts from. */
	template?: V
	/** Whatever else the editor needs from the response (permissions, the raw row). */
	meta?: unknown
}

export type ItemWriteContext<V> = ItemKey & {
	value: V
	/** `undefined` when the write must create rather than update. */
	deployed: V | undefined
	meta: unknown
}

export type ItemAdapter<V> = {
	load?: (key: ItemKey) => Promise<ItemLoad<V>>
	write?: (ctx: ItemWriteContext<V>) => Promise<void>
	/** Where a save puts `value`. Defaults to its `path` field. */
	pathOf?: (value: V) => string
	/**
	 * The form writes into the value on its own as it renders (schema defaults, normalization).
	 * With `settles`, those writes join `deployed` until `markEdited` — the user's first input —
	 * instead of reading as a draft. Only for an item loaded without a draft row.
	 */
	settles?: boolean
	/** Whether a pre-input change may be absorbed; one that may not counts as the first edit. */
	absorbs?: (next: V, deployed: V) => boolean
}

export type SaveOutcome =
	| { ok: true; path: string; moved: boolean }
	| { ok: false; error: string; skipped?: boolean }
export type CommandOutcome = { ok: true } | { ok: false; error: string }
export type DiscardOutcome = { removed: boolean }

/** The draft-row side: the existing debounced syncer, and the list pages' `*` hint. */
export type ItemRowPort = {
	write(key: ItemKey, value: unknown | null): void
	/** Send what `write` queued for `key` now; resolves once it landed. */
	flush(key: ItemKey): Promise<void>
	overwrite(key: ItemKey, value: unknown | null): Promise<void>
	seedSync(key: ItemKey, draftSavedAt: string | undefined): void
	conflicted(key: ItemKey): boolean
	failure(key: ItemKey): string | undefined
	dropPending(key: ItemKey): void
	hint(key: ItemKey, on: boolean): void
}

const TEMPORARY_PREFIX = 'draft:'

/** A fresh identity for an item being created, so it is an item like any other from birth. */
export function newItemPath(): string {
	return `${TEMPORARY_PREFIX}${randomUUID()}`
}

export function isTemporaryPath(path: string): boolean {
	return path.startsWith(TEMPORARY_PREFIX)
}

function isItemKind(kind: string): kind is ItemKind {
	return (ITEM_KINDS as readonly string[]).includes(kind)
}

function keyString(key: ItemKey): string {
	return `${key.workspace}/${key.kind}/${key.path}`
}

function serialize(value: unknown): string | undefined {
	return value === undefined ? undefined : JSON.stringify(value)
}

function snapshot<V>(value: V): V {
	if (value === undefined) return value
	try {
		return structuredClone($state.snapshot(value)) as V
	} catch {
		return JSON.parse(JSON.stringify(value)) as V
	}
}

function errorMessage(e: unknown): string {
	if (e == null) return 'Unknown error'
	if (typeof e === 'string') return e
	const err = e as { body?: unknown; message?: unknown }
	if (typeof err.body === 'string' && err.body) return err.body
	if (typeof err.message === 'string' && err.message) return err.message
	return String(e)
}

/** Revisions are unique across entries, so a consumer switching items never mistakes one
 *  item's revision for another's. */
let nextRevision = 1

class Entry<V> {
	key: ItemKey = $state()!
	value: V | undefined = $state()
	deployed: V | undefined = $state.raw()
	template: V | undefined = $state.raw()
	origin: ItemOrigin | undefined = $state()
	meta: unknown = $state.raw()
	loaded = $state(false)
	removed = $state(false)
	pristine = $state(false)
	/** Bumped whenever the value is replaced from outside the form — load, discard, an
	 *  external write — which is what a form not bound to the value re-reads it on. */
	revision = $state(0)
	error: string | undefined = $state()
	private loads = $state(0)
	private commands = $state(0)

	dirty = $derived.by(() => {
		if (!this.loaded || this.value === undefined) return false
		if (this.origin === 'draft') return true
		return !draftValuesEqual(this.value, this.origin === 'new' ? this.template : this.deployed)
	})

	busy = $derived(this.commands > 0)

	status: ItemStatus = $derived.by(() => {
		if (this.loads > 0 && !this.loaded) return 'loading'
		if (this.commands > 0) return 'saving'
		if (this.ports.conflicted(this.key)) return 'conflicted'
		if (this.error !== undefined || this.ports.failure(this.key) !== undefined) return 'failed'
		return 'idle'
	})

	settles = false
	absorbs: ((next: V, deployed: V) => boolean) | undefined
	refs = 0
	disposed = false
	/** Replaced by an entry that moved onto its key: it no longer owns the row there. */
	retired = false
	handles = new Set<Handle<V>>()
	stopWatch: (() => void) | undefined
	/** Fields a `patch` has put on the deployed side ahead of the server; a save landing
	 *  meanwhile keeps them rather than rolling the baseline back past them. */
	private patched: Record<string, unknown> = {}
	/** The value as last observed, serialized: `touch` acts only on a real change. */
	private seen: string | undefined
	/** The row last handed to the syncer (`null`: none, `undefined`: not known — the next
	 *  reconcile writes whatever the rule says). What `reconcile` diffs against. */
	private row: string | null | undefined = null
	/** Counts value changes, so a load can tell whether one landed while it was in flight. */
	private changes = 0
	private queue: Promise<unknown> = Promise.resolve()
	private ports: ItemRowPort
	private store: StoreInternals

	constructor(key: ItemKey, ports: ItemRowPort, store: StoreInternals) {
		this.ports = ports
		this.store = store
		this.key = key
	}

	/** The form's own write: replaces the value without counting as an external change. */
	setValue(value: V | undefined): void {
		this.value = value
		this.touch()
	}

	/** React to whatever the value holds now. Idempotent: an unchanged value does nothing. */
	touch(): void {
		if (!this.loaded) return
		const s = serialize(this.value)
		if (s === this.seen) return
		this.seen = s
		this.changes++
		if (this.pristine && this.origin === 'deployed' && this.value !== undefined) {
			const value = snapshot(this.value)
			if (!this.absorbs || this.deployed === undefined || this.absorbs(value, this.deployed)) {
				this.deployed = value
			} else {
				this.pristine = false
			}
		}
		this.reconcile()
	}

	/** Mirror `dirty ? value : null` to the draft row. The only place a row is written. */
	reconcile(): void {
		const key = this.key
		if (this.retired || !this.loaded || this.origin === 'new' || isTemporaryPath(key.path)) return
		const desired = this.dirty ? (serialize(this.value) ?? null) : null
		this.ports.hint(key, desired !== null)
		if (desired === this.row) return
		// A rejected write is not retried: the row stays as the server has it until the
		// conflict is resolved, by a reload or an overwrite.
		if (this.ports.conflicted(key)) {
			this.ports.dropPending(key)
			return
		}
		this.row = desired
		this.ports.write(key, desired === null ? null : snapshot(this.value))
	}

	private replaceValue(value: V | undefined): void {
		this.value = snapshot(value)
		this.seen = serialize(this.value)
		this.changes++
		this.revision = nextRevision++
	}

	private run<T>(fn: () => Promise<T>): Promise<T> {
		this.commands++
		this.refs++
		const result = this.queue.then(fn).finally(() => {
			this.commands--
			this.store.release(this)
		})
		this.queue = result.catch(() => {})
		return result
	}

	/** Count a command as started now, ahead of its turn in the queue; returns its release. */
	begin(): () => void {
		this.commands++
		this.refs++
		return () => {
			this.commands--
			this.store.release(this)
		}
	}

	initNew(template: V): void {
		this.template = snapshot(template)
		this.origin = 'new'
		this.loaded = true
		this.replaceValue(template)
	}

	load(adapter: ItemAdapter<V>): Promise<CommandOutcome> {
		this.loads++
		const changesAtStart = this.changes
		return this.run(async () => {
			const key = this.key
			let res: ItemLoad<V>
			try {
				if (!adapter.load) throw new Error('This item cannot be loaded')
				res = await adapter.load(key)
			} catch (e) {
				this.error = errorMessage(e)
				return { ok: false, error: this.error }
			} finally {
				this.loads--
			}
			// Whatever landed since the read was asked for — an external write, an edit — is
			// newer than what it read, so the read only moves the deployed side under it.
			const keepValue = this.changes !== changesAtStart
			this.meta = res.meta
			this.error = undefined
			if (isTemporaryPath(key.path)) {
				this.template = snapshot(res.template)
				this.origin = 'new'
				this.loaded = true
				if (!keepValue) this.replaceValue(res.template)
				return { ok: true }
			}
			this.deployed = snapshot(res.deployed)
			this.origin = res.deployed !== undefined ? 'deployed' : 'draft'
			this.row = serialize(res.draft) ?? null
			this.ports.seedSync(key, res.draftSavedAt)
			this.loaded = true
			if (!keepValue) {
				this.pristine = this.settles && this.origin === 'deployed' && res.draft === undefined
				this.replaceValue(res.draft ?? res.deployed)
			}
			this.removed = this.value === undefined
			this.reconcile()
			return { ok: true }
		})
	}

	/** An outside write (the AI chat, another editor): a real divergence, never settling. */
	applyExternal(value: V): number {
		if (this.loaded && serialize(value) === serialize(this.value)) return this.revision
		this.pristine = false
		this.removed = false
		this.replaceValue(value)
		this.reconcile()
		return this.revision
	}

	markEdited(): void {
		// Written only on the transition: removing a focused input fires a trusted `change` from
		// inside Svelte's DOM teardown, where writing any state throws `state_unsafe_mutation`.
		if (this.pristine) this.pristine = false
	}

	/** What a save asked for now would send: the value as the user has it at the click. */
	ask(): V | undefined {
		if (!this.loaded || this.value === undefined) return undefined
		this.touch()
		return snapshot(this.value)
	}

	/** `started`: the release `begin` handed out, for a save already counted as started. */
	save(adapter: ItemAdapter<V>, started?: () => void, asked = this.ask()): Promise<SaveOutcome> {
		const body = async (): Promise<SaveOutcome> => {
			const sent = asked ?? this.ask()
			if (sent === undefined) return { ok: false, error: this.error ?? 'Nothing to save' }
			const from = this.key
			// A create or a draft-only item is always a write; anything else already deployed is
			// saved — which is also what makes a second click behind an in-flight save a no-op.
			// Compared exactly, not as drafts are: a field the draft comparison ignores (a
			// schedule's run-as) is still one an explicit save must send.
			if (this.origin === 'deployed' && serialize(sent) === serialize(this.deployed)) {
				return { ok: true, path: from.path, moved: false }
			}
			const to = (adapter.pathOf ?? ((v: V) => (v as { path: string }).path))(sent)
			try {
				if (!adapter.write) throw new Error('This item cannot be saved')
				await adapter.write({
					...from,
					value: sent,
					deployed: this.origin === 'deployed' ? snapshot(this.deployed) : undefined,
					meta: this.meta
				})
			} catch (e) {
				this.error = errorMessage(e)
				return { ok: false, error: this.error }
			}
			this.error = undefined
			this.deployed = { ...sent, ...this.patched } as V
			this.origin = 'deployed'
			this.template = undefined
			const moved = to !== from.path
			if (moved) this.moveTo(to)
			this.reconcile()
			await this.settleRows(moved ? [from, this.key] : [this.key])
			return { ok: true, path: to, moved }
		}
		if (!started) return this.run(body)
		const result = this.queue.then(body).finally(started)
		this.queue = result.catch(() => {})
		return result
	}

	discard(): Promise<DiscardOutcome> {
		return this.run(async () => {
			if (!this.loaded) return { removed: false }
			if (this.origin === 'draft') {
				const kept = snapshot(this.value)
				const keptRow = this.row
				this.replaceValue(undefined)
				this.reconcile()
				await this.settleRows([this.key])
				// The row is the item: while its delete has not landed, the item is still there.
				if (this.rowRefused(this.key)) {
					// The syncer keeps a failed payload for its page-close flush; that delete would
					// remove the item this reports as kept.
					this.ports.dropPending(this.key)
					this.row = keptRow
					this.replaceValue(kept)
					this.reconcile()
					return { removed: false }
				}
				this.removed = true
				return { removed: true }
			}
			this.pristine = this.settles && this.origin === 'deployed'
			this.replaceValue(this.origin === 'new' ? this.template : this.deployed)
			this.reconcile()
			await this.settleRows([this.key])
			return { removed: false }
		})
	}

	/**
	 * A command that changes what exists resolves once the rows it implied have landed, so a
	 * list re-read on its outcome sees neither the draft a save made redundant nor a draft-only
	 * item that is gone.
	 */
	private async settleRows(keys: ItemKey[]): Promise<void> {
		await Promise.all(keys.filter((k) => !isTemporaryPath(k.path)).map((k) => this.ports.flush(k)))
	}

	/** The syncer settles a rejected or failed write without throwing; this is how to tell. */
	private rowRefused(key: ItemKey): boolean {
		return this.ports.conflicted(key) || this.ports.failure(key) !== undefined
	}

	/**
	 * Re-key to `path`. The row at the path left behind follows from nothing living there. The
	 * one at `path` was never read: whatever it holds predates the write that just landed there,
	 * so it is reconciled like any other.
	 */
	private moveTo(path: string): void {
		const from = this.key
		if (!isTemporaryPath(from.path)) {
			this.ports.hint(from, false)
			if (this.row !== null) this.ports.write(from, null)
		}
		this.row = undefined
		this.key = { ...from, path }
		this.store.rekey(this, from)
	}

	/**
	 * Write `fields` to the server through `write`. Both sides take them at once — the value
	 * because the user asked, the baseline because the server is about to hold them — so the
	 * request never reads as a draft. On failure each side gives back those it still holds.
	 */
	patch(fields: Partial<V>, write: (key: ItemKey) => Promise<unknown>): Promise<CommandOutcome> {
		this.pristine = false
		const baseKey = this.origin === 'new' ? 'template' : 'deployed'
		const before = snapshot(this[baseKey]) as Record<string, unknown> | undefined
		const sent = snapshot(fields) as Record<string, unknown>
		if (before !== undefined) this[baseKey] = { ...before, ...sent } as V
		if (baseKey === 'deployed') Object.assign(this.patched, sent)
		if (this.value !== undefined) Object.assign(this.value as object, fields)
		this.touch()
		const settled = () => {
			for (const [k, v] of Object.entries(sent)) {
				if (deepEqual(this.patched[k], v)) delete this.patched[k]
			}
		}
		return this.run(async () => {
			try {
				await write(this.key)
			} catch (e) {
				settled()
				const giveBack = (side: V | undefined): V | undefined => {
					if (side === undefined || before === undefined) return undefined
					const out = snapshot(side) as Record<string, unknown>
					let changed = false
					for (const [k, v] of Object.entries(sent)) {
						if (deepEqual(out[k], v)) {
							out[k] = before[k]
							changed = true
						}
					}
					return changed ? (out as V) : undefined
				}
				const base = giveBack(this[baseKey])
				if (base !== undefined) this[baseKey] = base
				const value = giveBack(this.value)
				if (value !== undefined) this.replaceValue(value)
				this.error = errorMessage(e)
				this.reconcile()
				return { ok: false, error: this.error }
			}
			settled()
			this.error = undefined
			if (this[baseKey] !== undefined) this[baseKey] = { ...this[baseKey], ...sent } as V
			this.touch()
			this.reconcile()
			return { ok: true }
		})
	}

	/** Fetch something about the deployed item the load did not carry (a secret's plaintext),
	 *  and write it into both sides. */
	learn(fetch: (key: ItemKey) => Promise<(side: V) => void>): Promise<CommandOutcome> {
		return this.run(async () => {
			let apply: (side: V) => void
			try {
				apply = await fetch(this.key)
			} catch (e) {
				return { ok: false, error: errorMessage(e) }
			}
			if (this.deployed !== undefined) {
				const d = snapshot(this.deployed)
				apply(d)
				this.deployed = d
			}
			if (this.value !== undefined) {
				const v = snapshot(this.value)
				apply(v)
				this.replaceValue(v)
			}
			this.reconcile()
			return { ok: true }
		})
	}

	/** Out of `conflicted`: `reload` takes the server's row, `overwrite` forces this one. */
	resolveConflict(how: 'reload' | 'overwrite', adapter: ItemAdapter<V>): Promise<CommandOutcome> {
		// The syncer keeps a rejected payload parked, and the page-close flush would send it with
		// the timestamp the reload adopts — over the version the user just chose.
		if (how === 'reload') {
			this.ports.dropPending(this.key)
			return this.load(adapter)
		}
		return this.run(async () => {
			const desired = this.dirty ? snapshot(this.value) : null
			await this.ports.overwrite(this.key, desired)
			this.row = serialize(desired ?? undefined) ?? null
			this.reconcile()
			return { ok: true }
		})
	}
}

type StoreInternals = {
	release(entry: Entry<any>): void
	rekey(entry: Entry<any>, from: ItemKey): void
}

export type ItemHandle<V> = {
	readonly key: ItemKey
	/** What the user is editing. Assign or mutate in place; both reach the draft row. */
	value: V | undefined
	/** Last known server value; `undefined` for a draft-only or new item. */
	readonly deployed: V | undefined
	readonly origin: ItemOrigin | undefined
	readonly status: ItemStatus
	readonly error: string | undefined
	readonly loaded: boolean
	/** A draft-only item its discard removed. */
	readonly removed: boolean
	readonly dirty: boolean
	/** A command is queued or running. */
	readonly busy: boolean
	readonly pristine: boolean
	readonly revision: number
	readonly meta: unknown
	/** `dirty && valid && writable && !busy`. */
	readonly canSave: boolean
	save(): Promise<SaveOutcome>
	discard(): Promise<DiscardOutcome>
	reload(): Promise<CommandOutcome>
	applyExternal(value: V): number
	markEdited(): void
	patch(fields: Partial<V>, write: (key: ItemKey) => Promise<unknown>): Promise<CommandOutcome>
	learn(fetch: (key: ItemKey) => Promise<(side: V) => void>): Promise<CommandOutcome>
	resolveConflict(how: 'reload' | 'overwrite'): Promise<CommandOutcome>
}

export type ItemSpec<V> = {
	workspace: string | undefined
	/** `undefined`: no item. A temporary path (`newItemPath`) opens a create. */
	path: string | undefined
	/** A create's starting value, when it is known up front. Read on first acquire only. */
	template?: V
	/** Tells two openings of the same item apart: a new value lets an idle item be read afresh. */
	session?: unknown
	valid?: () => boolean
	writable?: () => boolean
}

/** A class, not a literal: Svelte deep-proxies plain objects put in `$state`, and a proxied
 *  handle would no longer be the one `saveEach` recognizes. */
class Handle<V> implements ItemHandle<V> {
	/** Reactive: a handle follows its item onto the entry that took over its key. */
	entry: Entry<V> = $state.raw()!
	readonly spec: ItemSpec<V>
	readonly adapter: ItemAdapter<V>

	constructor(entry: Entry<V>, spec: ItemSpec<V>, adapter: ItemAdapter<V>) {
		this.entry = entry
		this.spec = spec
		this.adapter = adapter
		entry.handles.add(this)
	}

	get key() {
		return this.entry.key
	}
	get value() {
		return this.entry.value
	}
	set value(v) {
		this.entry.setValue(v)
	}
	get deployed() {
		return this.entry.deployed
	}
	get origin() {
		return this.entry.origin
	}
	get status() {
		return this.entry.status
	}
	get error() {
		return this.entry.error
	}
	get loaded() {
		return this.entry.loaded
	}
	get removed() {
		return this.entry.removed
	}
	get dirty() {
		return this.entry.dirty
	}
	get busy() {
		return this.entry.busy
	}
	get pristine() {
		return this.entry.pristine
	}
	get revision() {
		return this.entry.revision
	}
	get meta() {
		return this.entry.meta
	}
	get canSave() {
		return (
			this.entry.dirty &&
			(this.spec.valid?.() ?? true) &&
			(this.spec.writable?.() ?? true) &&
			!this.entry.busy
		)
	}
	save() {
		return this.entry.save(this.adapter)
	}
	discard() {
		return this.entry.discard()
	}
	reload() {
		return this.entry.load(this.adapter)
	}
	applyExternal(value: V) {
		return this.entry.applyExternal(value)
	}
	markEdited() {
		this.entry.markEdited()
	}
	patch(fields: Partial<V>, write: (key: ItemKey) => Promise<unknown>) {
		return this.entry.patch(fields, write)
	}
	learn(fetch: (key: ItemKey) => Promise<(side: V) => void>) {
		return this.entry.learn(fetch)
	}
	resolveConflict(how: 'reload' | 'overwrite') {
		return this.entry.resolveConflict(how, this.adapter)
	}
}

export type ItemAcquisition<V> = { handle: ItemHandle<V>; release(): void }

export function createItemStore(ports: ItemRowPort) {
	const entries = new Map<string, Entry<any>>()

	const internals: StoreInternals = {
		release(entry) {
			entry.refs--
			if (entry.refs > 0 || entry.busy) return
			entry.touch()
			entry.stopWatch?.()
			entry.disposed = true
			const k = keyString(entry.key)
			if (entries.get(k) === entry) entries.delete(k)
		},
		rekey(entry, from) {
			const k = keyString(from)
			if (entries.get(k) === entry) entries.delete(k)
			const to = keyString(entry.key)
			const displaced = entries.get(to)
			if (displaced && displaced !== entry) retire(displaced, entry)
			entries.set(to, entry)
		}
	}

	/**
	 * An entry moved onto a key another live entry holds. One key has one row writer, so the
	 * old one stops writing and every handle on it moves to the entry that now is the item —
	 * the same outcome as when nobody had it open: what was there is superseded by the write.
	 */
	function retire(old: Entry<any>, into: Entry<any>): void {
		old.retired = true
		old.stopWatch?.()
		for (const handle of old.handles) {
			handle.entry = into
			into.handles.add(handle)
			into.refs++
			old.refs--
		}
		old.handles.clear()
	}

	function watch(entry: Entry<any>): void {
		// Detached from whichever component acquired it first: the entry outlives that component
		// while another holder, or a command in flight, still has it.
		queueMicrotask(() => {
			if (entry.disposed) return
			entry.stopWatch = $effect.root(() => {
				$effect(() => {
					const value = entry.value
					if (value !== undefined) readFieldsRecursively(value)
					untrack(() => entry.touch())
				})
			})
		})
	}

	/** Take a reference on the item at `key`, creating (and loading) it if nobody holds it. */
	function acquire<V>(
		key: ItemKey,
		spec: ItemSpec<V>,
		adapter: ItemAdapter<V>
	): ItemAcquisition<V> {
		const k = keyString(key)
		let entry = entries.get(k) as Entry<V> | undefined
		if (!entry) {
			entry = new Entry<V>(key, ports, internals)
			entry.settles = adapter.settles ?? false
			entry.absorbs = adapter.absorbs
			entries.set(k, entry)
			if (isTemporaryPath(key.path) && spec.template !== undefined) entry.initNew(spec.template)
			else void entry.load(adapter)
			watch(entry)
		}
		entry.refs++
		const handle = new Handle(entry, spec, adapter)
		let released = false
		return {
			handle,
			release() {
				if (released) return
				released = true
				handle.entry.handles.delete(handle)
				internals.release(handle.entry)
			}
		}
	}

	/**
	 * Save each item in turn, stopping at the first failure. All of them read as busy from the
	 * call on, not only once their turn comes.
	 */
	function saveEach<V>(items: ItemHandle<V>[]): Promise<SaveOutcome[]> {
		const pending = items.map((item) => {
			// `useItem` hands out a forwarder; the item it forwards to is its `current`.
			const handle = item instanceof Handle ? item : (item as { current?: unknown }).current
			if (!(handle instanceof Handle)) throw new Error('saveEach takes handles from useItem(s)')
			const { entry, adapter } = handle as Handle<V>
			return { entry, adapter, started: entry.begin(), asked: entry.ask() }
		})
		return (async () => {
			const outcomes: SaveOutcome[] = []
			let failed = false
			for (const { entry, adapter, started, asked } of pending) {
				if (failed) {
					started()
					outcomes.push({ ok: false, error: 'Not saved', skipped: true })
					continue
				}
				const outcome = await entry.save(adapter, started, asked)
				outcomes.push(outcome)
				failed = !outcome.ok
			}
			return outcomes
		})()
	}

	function find(workspace: string, kind: string, path: string): Entry<any> | undefined {
		if (!isItemKind(kind)) return undefined
		return entries.get(keyString({ workspace, kind, path }))
	}

	const bridge = {
		seed(workspace: string, kind: UserDraftItemKind, path: string, value: unknown): boolean {
			const entry = find(workspace, kind, path)
			if (!entry || value === undefined || value === null) return false
			entry.applyExternal(value)
			return true
		},
		/** The draft as this tab knows it: the value when it diverges, nothing otherwise. */
		read(
			workspace: string,
			kind: UserDraftItemKind,
			path: string
		): { value: unknown | undefined } | undefined {
			const entry = find(workspace, kind, path)
			if (!entry || !entry.loaded) return undefined
			return { value: entry.dirty && entry.origin !== 'new' ? snapshot(entry.value) : undefined }
		},
		discard(workspace: string, kind: UserDraftItemKind, path: string): boolean {
			const entry = find(workspace, kind, path)
			if (!entry) return false
			void entry.discard()
			return true
		},
		list(workspace: string, kinds: readonly UserDraftItemKind[]): UserDraftEntry[] {
			const out: UserDraftEntry[] = []
			for (const entry of entries.values()) {
				const key = entry.key
				if (key.workspace !== workspace || !kinds.includes(key.kind)) continue
				if (!entry.loaded || !entry.dirty || entry.origin === 'new') continue
				out.push({
					workspace: key.workspace,
					itemKind: key.kind,
					path: key.path,
					value: snapshot(untrack(() => entry.value))
				})
			}
			return out
		}
	}

	return { acquire, saveEach, bridge }
}

function keyQuery(key: ItemKey) {
	return { workspace: key.workspace, itemKind: key.kind, path: key.path }
}

const syncerRows: ItemRowPort = {
	write(key, value) {
		void UserDraftDbSyncer.save({ ...keyQuery(key), value })
	},
	flush(key) {
		return UserDraftDbSyncer.flush(keyQuery(key))
	},
	overwrite(key, value) {
		return UserDraftDbSyncer.overwrite({ ...keyQuery(key), value })
	},
	seedSync(key, draftSavedAt) {
		UserDraftDbSyncer.recordRemoteSync(keyQuery(key), draftSavedAt)
	},
	conflicted(key) {
		return UserDraftDbSyncer.getConflict(keyQuery(key)).conflict !== undefined
	},
	failure(key) {
		const state = UserDraftDbSyncer.getState(keyQuery(key))
		return state.state === 'failed' ? (state.failureMessage ?? 'Draft save failed') : undefined
	},
	dropPending(key) {
		UserDraftDbSyncer.dropPending(keyQuery(key))
	},
	hint(key, on) {
		setLocalDraftHint(key.workspace, key.kind, key.path, on)
	}
}

export const itemStore = createItemStore(syncerRows)
registerLiveItemBridge(itemStore.bridge)

/**
 * Hold one item per spec, aligned with `getSpecs()`: re-keying acquires the new item and
 * releases the old; a rename made by a save keeps the handle, which follows its item. Must be
 * called during component init.
 */
export function useItems<V>(
	kind: ItemKind,
	getSpecs: () => ItemSpec<V>[],
	adapter: ItemAdapter<V>
): (ItemHandle<V> | undefined)[] {
	const handles = $state<(ItemHandle<V> | undefined)[]>([])
	const held = new Map<string, ItemAcquisition<V>>()

	function reconcile() {
		const specs = getSpecs()
		const wanted: { id: string; key: ItemKey; spec: ItemSpec<V> }[] = []
		for (const spec of specs) {
			if (!spec.workspace || !spec.path) {
				wanted.push({ id: '', key: undefined as unknown as ItemKey, spec })
				continue
			}
			const key: ItemKey = { workspace: spec.workspace, kind, path: spec.path }
			wanted.push({ id: `${keyString(key)}#${String(spec.session ?? '')}`, key, spec })
		}
		const ids = new Set(wanted.map((w) => w.id))
		// Release first, so an item reopened under a new session is read afresh when idle.
		for (const [id, acq] of [...held]) {
			if (!ids.has(id)) {
				acq.release()
				held.delete(id)
			}
		}
		const next = wanted.map(({ id, key, spec }) => {
			if (!id) return undefined
			let acq = held.get(id)
			if (!acq) {
				acq = itemStore.acquire(key, spec, adapter)
				held.set(id, acq)
			}
			return acq.handle
		})
		untrack(() => {
			const same = handles.length === next.length && handles.every((h, i) => h === next[i])
			if (!same) handles.splice(0, handles.length, ...next)
		})
	}

	untrack(reconcile)
	// Before the template renders: a spec that moved on must not render one more frame of the
	// item it left, remounting that item's form for nothing.
	$effect.pre(reconcile)
	onDestroy(() => {
		for (const acq of held.values()) acq.release()
		held.clear()
	})
	return handles
}

const noItem = (): Promise<{ ok: false; error: string }> =>
	Promise.resolve({ ok: false, error: 'No item' })

/** Single-item `useItems`, as a stable object that forwards to whichever item is current. */
export function useItem<V>(
	kind: ItemKind,
	getSpec: () => ItemSpec<V>,
	adapter: ItemAdapter<V>
): ItemHandle<V> & { readonly current: ItemHandle<V> | undefined } {
	const handles = useItems<V>(kind, () => [getSpec()], adapter)
	const h = () => handles[0]
	return {
		get current() {
			return h()
		},
		get key() {
			return h()?.key as ItemKey
		},
		get value() {
			return h()?.value
		},
		set value(v) {
			const handle = h()
			if (handle) handle.value = v
		},
		get deployed() {
			return h()?.deployed
		},
		get origin() {
			return h()?.origin
		},
		get status() {
			return h()?.status ?? 'idle'
		},
		get error() {
			return h()?.error
		},
		get loaded() {
			return h()?.loaded ?? false
		},
		get removed() {
			return h()?.removed ?? false
		},
		get dirty() {
			return h()?.dirty ?? false
		},
		get busy() {
			return h()?.busy ?? false
		},
		get pristine() {
			return h()?.pristine ?? false
		},
		get revision() {
			return h()?.revision ?? 0
		},
		get meta() {
			return h()?.meta
		},
		get canSave() {
			return h()?.canSave ?? false
		},
		save: () => h()?.save() ?? noItem(),
		discard: () => h()?.discard() ?? Promise.resolve({ removed: false }),
		reload: () => h()?.reload() ?? noItem(),
		applyExternal: (v) => h()?.applyExternal(v) ?? 0,
		markEdited: () => h()?.markEdited(),
		patch: (fields, write) => h()?.patch(fields, write) ?? noItem(),
		learn: (fetch) => h()?.learn(fetch) ?? noItem(),
		resolveConflict: (how) => h()?.resolveConflict(how) ?? noItem()
	}
}

/** `saveEach` over the shared store. */
export function saveEach<V>(items: ItemHandle<V>[]): Promise<SaveOutcome[]> {
	return itemStore.saveEach(items)
}
