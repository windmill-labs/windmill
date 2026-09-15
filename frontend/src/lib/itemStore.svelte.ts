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
	/** The deployed item this value describes, when it is not the one at `path`. */
	standsFor: string | undefined
	meta: unknown
}

export type ItemAdapter<V> = {
	load?: (key: ItemKey) => Promise<ItemLoad<V>>
	/** Resolves to what the server holds after the write, when that is not `value`: an endpoint
	 *  that sets a field on its own (a schedule create enabling it) would otherwise be recorded
	 *  as holding what was sent. */
	write?: (ctx: ItemWriteContext<V>) => Promise<V | void>
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
	/** Send what `write` queued for `key` now. Resolves once the attempt has settled, which is
	 *  not the same as the row being there: a conflict or a network failure resolves too, and
	 *  shows up in `conflicted` / `failure` for the caller to read. */
	flush(key: ItemKey): Promise<void>
	overwrite(key: ItemKey, value: unknown | null): Promise<void>
	/** Rows handed to the syncer for `key` so far, by any writer, counted before the debounce
	 *  rather than on the response. Handed back to `seedSync` to order it. */
	rowMark(key: ItemKey): number
	/** A row waiting for another attempt, if any: `undefined` when none, `null` for a delete. */
	pending(key: ItemKey): unknown | undefined
	/** Whether a row is waiting with no baseline behind it, so sending it would be unconditional. */
	unbased(key: ItemKey): boolean
	/** Raise a conflict the server has not reported, for the user to resolve. */
	markConflict(key: ItemKey, serverTimestamp: string): void
	seedSync(key: ItemKey, draftSavedAt: string | undefined, since: number): void
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
/** Identifies which patch owns a field it put ahead of the server: two patches of one field can
 *  carry the same value, so ownership cannot be told from the value. */
let nextPatch = 1

const superseded = { ok: false, error: 'Another save of this item replaced this one' } as const
const outOfDate = {
	ok: false,
	error: 'This item changed elsewhere and could not be re-read. Reload it before saving.'
} as const

/** What a command did not do, so it can tell what moved under it. `edits`: what the user can see
 *  change, which a discard measures against — an outside write of the value already shown does
 *  not cancel one. `externals`: every outside write, that one included, since it made the value a
 *  row a read in flight has not got. */
type AsOf = { edits: number; externals: number }

class Entry<V> {
	key: ItemKey = $state()!
	value: V | undefined = $state()
	/** What the server last told us this item holds. */
	private serverDeployed: V | undefined = $state.raw()
	/** Fields a `patch` has put ahead of the server, each tagged with the patch that owns it,
	 *  until that patch lands or is refused. */
	private patched: Record<string, { by: number; value: unknown }> = $state({})
	/**
	 * The baseline the value is measured against: what the server holds, with whatever a patch
	 * is currently ahead of it on. Derived rather than assigned, so that no writer of the server
	 * side can forget the overlay and roll the baseline back past an optimistic toggle — which
	 * reads as an unsaved change to a field the user never touched.
	 */
	deployed: V | undefined = $derived.by(() => {
		if (this.serverDeployed === undefined) return undefined
		const out = { ...this.serverDeployed } as Record<string, unknown>
		for (const [field, held] of Object.entries(this.patched)) out[field] = held.value
		return out as V
	})
	template: V | undefined = $state.raw()
	origin: ItemOrigin | undefined = $state()
	meta: unknown = $state.raw()
	loaded = $state(false)
	removed = $state(false)
	/** A write landed on this item elsewhere and the re-read meant to pick it up failed, so
	 *  `deployed` is known to be behind the server. Saving or discarding against it would put
	 *  that back over the write, so neither is offered until a reload clears this. */
	stale = $state(false)
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
	/** The deployed item this one's value describes, for an entry kept under a temporary key. */
	standsFor: string | undefined
	/** How this item is read and written, from whoever opened it first. */
	adapter: ItemAdapter<V> | undefined
	refs = 0
	disposed = false
	/** Replaced by an entry that moved onto its key: it no longer owns the row there, and a write
	 *  reaching its turn does nothing, as its handles show the item that replaced it. */
	retired = false
	/** The command running now, past its turn: what a move onto this key waits for. */
	running: Promise<unknown> | undefined
	/** The entries a save of this entry waits for before moving: a move skips waiting for any
	 *  that waits for it in turn, which would never end. */
	waitingOn: Entry<any>[] = []
	handles = new Set<Handle<V>>()
	stopWatch: (() => void) | undefined
	/** The value as last observed, serialized: `touch` acts only on a real change. */
	private seen: string | undefined
	/** The row last handed to the syncer (`null`: none, `undefined`: not known — the next
	 *  reconcile writes whatever the rule says). What `reconcile` diffs against. */
	private row: string | null | undefined = null
	/** Counts what the user typed and what an outside writer put here, so a read can tell whether
	 *  one landed while it was in flight. A command's own replacement of the value is not one:
	 *  a discard that ran meanwhile is the answer to a question the read is not asking. */
	private edits = 0
	/** Counts outside writes, the ones that change nothing on screen included: those still put the
	 *  value on the server as a row, which a read already in flight knows nothing about. */
	private externals = 0
	/** Counts the rows this entry can account for: the ones it wrote itself, and the ones a caller
	 *  says it wrote for a value handed in here. A row left over after that is a row nobody here
	 *  has the value of. */
	private values = 0
	/** Whether the discard that last ran had anything to say about the key's row — it had one, or
	 *  it is deliberately keeping the one a newer edit put there. Decided inside the command, so
	 *  after whatever load was queued in front of it, not before. */
	private ownsRow = false
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
		this.edits++
		if (this.pristine && this.origin === 'deployed' && this.value !== undefined) {
			const value = snapshot(this.value)
			if (!this.absorbs || this.deployed === undefined || this.absorbs(value, this.deployed)) {
				this.serverDeployed = value
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
		// Known to be behind the server: writing from here would put that back over whatever is
		// actually there. A reload is what clears it.
		if (this.stale) return
		const desired = this.dirty ? (serialize(this.value) ?? null) : null
		this.ports.hint(key, desired !== null)
		// A rejected write is not retried: the row stays as the server has it until the conflict
		// is resolved, by a reload or an overwrite. Checked before the row is compared, because a
		// payload parked for retry has to go whether or not it matches what this thinks is there
		// — a later flush would send it otherwise, deciding the conflict on the user's behalf.
		if (this.ports.conflicted(key)) {
			this.ports.dropPending(key)
			return
		}
		if (desired === this.row) return
		this.row = desired
		this.values++
		this.ports.write(key, desired === null ? null : snapshot(this.value))
	}

	/** The row a read reported, as both the baseline `reconcile` diffs against and the syncer's
	 *  `last_sync`: one fact, so taken together and against one `asOf`, the rows sent when the
	 *  read was issued. Seeding behind a row sent since puts `last_sync` before what the syncer
	 *  has already sent, which the server refuses from then on. */
	private adoptRow(
		key: ItemKey,
		row: unknown,
		draftSavedAt: string | undefined,
		asOf: number,
		waiting: boolean
	) {
		if (this.ports.rowMark(key) !== asOf) return
		// A payload the server *refused* predates this read and the baseline taken below would
		// make it acceptable, so it goes; the reconcile after the read re-queues whatever the
		// value needs. One merely unsent — a network failure the syncer parks to retry — is the
		// only record of an edit that never reached anyone, so it stays.
		if (this.ports.conflicted(key)) this.ports.dropPending(key)
		this.row = row === null || row === undefined ? null : (serialize(row) ?? null)
		// A payload still waiting to be sent was written against the baseline it had then. Taking
		// this read's would make it acceptable over whatever has been written since, so it would
		// overwrite that without ever conflicting. Left alone, its own baseline decides.
		if (!waiting) this.ports.seedSync(key, draftSavedAt, asOf)
	}

	private replaceValue(value: V | undefined): void {
		this.value = snapshot(value)
		this.seen = serialize(this.value)
		this.revision = nextRevision++
	}

	/**
	 * Run a command, handing it the state as of the moment it was asked for. Taken here rather
	 * than by each command, because the moment that matters is this one and not the command's
	 * turn, which is however long the queue ahead of it takes: whatever moved in between is
	 * newer than the command and is the command's to defer to.
	 */
	private run<T>(fn: (at: AsOf) => Promise<T>, asOf?: AsOf): Promise<T> {
		const at = asOf ?? this.asOf()
		this.commands++
		this.refs++
		const result = this.queue
			.then(() => this.turn(() => fn(at)))
			.finally(() => {
				this.commands--
				this.store.release(this)
			})
		this.queue = result.catch(() => {})
		return result
	}

	/** A command's turn: after any other entry's move onto this key, whichever entry was here
	 *  first, arrived during it or was put here by an earlier move. */
	private async turn<T>(fn: () => Promise<T>): Promise<T> {
		await this.store.moveSettled(this)
		const running = fn()
		this.running = running
		try {
			return await running
		} finally {
			if (this.running === running) this.running = undefined
		}
	}

	/** The registers that move on their own: what the user is editing, and the rows already sent.
	 *  A command compares against these to tell what has happened since it was asked for. */
	private asOf(): AsOf {
		return { edits: this.edits, externals: this.externals }
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

	/** `editedBefore`: the edit count as of when this read was decided on, for a caller that did
	 *  something async first. */
	load(adapter: ItemAdapter<V>, asOf?: AsOf, unasked = false): Promise<CommandOutcome> {
		this.loads++
		return this.run(async (at) => {
			const key = this.key
			let rowsAtRead = 0
			let valuesAtRead = 0
			let unbasedPending = false
			let refusedBefore = false
			let waitingBefore: unknown | undefined
			let res: ItemLoad<V>
			try {
				if (this.retired) return superseded
				if (!adapter.load) throw new Error('This item cannot be loaded')
				// A row queued by whoever held this key before is debounced: send it, or this read
				// answers with the deployed value and the draft reappears under a clean editor.
				// Not one written when no row existed, though — it carries no baseline, so it
				// would go out unconditional and take over a row created since. The read below
				// gives it one, and `pending` keeps showing it meanwhile.
				unbasedPending = this.ports.unbased(key)
				// Whether the server had already refused this key before this read went near it.
				refusedBefore = this.ports.conflicted(key)
				if (!unbasedPending) await this.ports.flush(key)
				// Taken here, not when the command was asked for: rows this read's own flush sent
				// are in the response it is about to get. Only one handed over from now on, while
				// the read is out, leaves that response behind.
				rowsAtRead = this.ports.rowMark(key)
				// Alongside the rows, not at the command's turn: a value taken during the flush
				// above already has its row on this side of the count, and crediting it again
				// would pay for a row that arrives later and belongs to nobody here.
				valuesAtRead = this.values
				// Taken before the read, not after: a row waiting here may land while the read is
				// out, and then the response predates it while `pending` has gone quiet. Read
				// afterwards, this entry would answer with the older value and reseed over the
				// baseline that row's own save had just set.
				waitingBefore = this.ports.pending(key)
				res = await adapter.load(key)
			} catch (e) {
				this.error = errorMessage(e)
				return { ok: false, error: this.error }
			} finally {
				this.loads--
			}
			// A conflicted key holds a value that is on screen only: the server refused it and
			// nothing can send it, so a read nobody asked for must not put the server's value in
			// its place, nor answer the conflict on the user's behalf. Checked here rather than
			// before the read, because the flush that precedes one is itself a way to find out.
			const standOff = unasked && this.ports.conflicted(key)
			// Whatever landed since the read was asked for — an external write, an edit — is
			// newer than what it read, so the read only moves the deployed side under it.
			const keepValue = standOff || this.edits !== at.edits || this.externals !== at.externals
			// More rows appeared while the read was out than values arrived to account for them,
			// so one was written by someone this entry cannot see — and it is later than anything
			// held here. Writing from this would post an older value over that row, on the
			// baseline it advanced, so nothing goes out and nothing is saved until a reload.
			this.stale = this.ports.rowMark(key) - rowsAtRead > this.values - valuesAtRead
			this.meta = res.meta
			this.error = undefined
			if (isTemporaryPath(key.path)) {
				this.template = snapshot(res.template)
				this.origin = 'new'
				this.loaded = true
				if (!keepValue) this.replaceValue(res.template)
				return { ok: true }
			}
			this.serverDeployed = snapshot(res.deployed)
			this.origin = res.deployed !== undefined ? 'deployed' : 'draft'
			// A row the syncer took but could not send is newer than this response and is the only
			// copy of that edit, so the item opens on it. One the server *refused* is the other
			// way round — the response is what won — and `adoptRow` drops it below.
			// A payload the server refused *before* this read is stale: the response is what won.
			// One refused by this read's own retry is not — it is the user's unsent edit, and the
			// only copy of it, so it stays on screen with the conflict for them to settle.
			const unsent = refusedBefore ? undefined : waitingBefore
			// A parked delete is what this tab wants gone, not what is there: the row is still the
			// server's, so the reconcile at the end re-issues the delete instead of believing it
			// landed and leaving the draft behind with nothing to remove it.
			const row = unsent == null ? res.draft : unsent
			const draft = (unsent === null ? undefined : row) as V | undefined
			// The deployed side above always advances: the item did change, and a discard after
			// the user keeps theirs has to land on what the server holds now.
			if (!standOff) this.adoptRow(key, row, res.draftSavedAt, rowsAtRead, unsent !== undefined)
			this.loaded = true
			if (!keepValue) {
				this.pristine = this.settles && this.origin === 'deployed' && draft === undefined
				this.replaceValue(draft ?? res.deployed)
			}
			this.removed = this.value === undefined
			// Raised after the value above has been taken, so what is on screen is still the
			// user's own unsent edit and "keep mine" means theirs: a payload written when this
			// key had no row, meeting a row that exists now. Sending it would overwrite that
			// unconditionally, and basing it on this read would claim it was built on a draft it
			// never saw — so neither happens until the user says which wins. Asked again here
			// rather than taken from before the read: that payload may have landed while the read
			// was out, and then it has a baseline of its own and the row is its own.
			if (this.ports.unbased(key) && res.draft !== undefined && res.draftSavedAt !== undefined) {
				this.ports.markConflict(key, res.draftSavedAt)
			}
			this.reconcile()
			return { ok: true }
		}, asOf)
	}

	/** Read the item again after someone else wrote it. The edits on screen are in the draft row,
	 *  so they come back over what was written; one typed since the read was asked for stays. */
	async reread(): Promise<CommandOutcome> {
		if (!this.loaded || this.retired || !this.adapter) return { ok: true }
		// Taken before the row goes, not after it lands: an edit typed while that write is in
		// flight is in neither it nor the read that follows, and is newer than both.
		const asOf = this.asOf()
		await this.ports.flush(this.key)
		return this.load(this.adapter, asOf, true)
	}

	/** Re-read for a caller that has just written the deployed item, after whatever is queued:
	 *  during the first load `reread` would find nothing loaded and do nothing. `absent` means
	 *  this editor never had the item, so the row is the caller's; `failed` means it has the item
	 *  on a stale baseline, and anything resetting to that would undo the write. */
	async refreshed(): Promise<'done' | 'absent' | 'failed'> {
		await this.run(async () => {})
		if (this.retired || !this.adapter) return 'absent'
		if (!this.loaded) return 'absent'
		// A read that answers but leaves the item behind an unseen row has not refreshed it
		// either: its caller must not go on believing this editor is current.
		if ((await this.reread()).ok && !this.stale) return 'done'
		// Same position as a failed `changed` re-read: the item moved and this did not see it.
		this.stale = true
		return 'failed'
	}

	/** Discard for an outside caller, called now so `discard` snapshots now: an edit made while it
	 *  queues must not look like part of what it was asked to throw away. `absent`: never had the
	 *  item, so the row is the caller's. `failed`: has it, on a baseline it cannot refresh. */
	async discarded(): Promise<'done' | 'absent' | 'failed'> {
		const asked = this.asOf()
		// A row this entry has none of is not one it can discard: it does nothing, and the caller
		// would read that as the row being gone while another tab's is still there.
		await this.discard(asked)
		if (this.retired || !this.loaded) return 'absent'
		if (!this.stale) return this.ownsRow ? 'done' : 'absent'
		// Stale refused it. A reload is what clears that, and with a baseline it can trust the
		// discard means something again — so the one thing worth trying before giving up. The
		// retry carries the state as of the request, so an edit typed during that reload stands.
		const askedAbout = serialize(this.value)
		const editsBefore = this.edits
		if (!this.adapter || !(await this.load(this.adapter)).ok) return 'failed'
		// The recovery turned up a different draft — changed by the read, not by anyone typing,
		// which is what `edits` tells apart. The retry would take that for what it was asked to
		// throw away and delete someone else's newer row, past the conflict its own baseline
		// would have raised. An edit made meanwhile is the other case, and `discard` handles it.
		if (serialize(this.value) !== askedAbout && this.edits === editsBefore) return 'failed'
		await this.discard(asked)
		return this.stale ? 'failed' : this.ownsRow ? 'done' : 'absent'
	}

	/**
	 * The deployed item itself was deleted elsewhere. Not a discard: there is no baseline left to
	 * reset to, so the editor reports it gone rather than sitting clean on something whose next
	 * save would 404. The server removes the row with the item, so none is written.
	 */
	itemDeleted(): Promise<void> {
		return this.run(async () => {
			if (this.retired || !this.loaded) return
			this.serverDeployed = undefined
			this.patched = {}
			this.replaceValue(undefined)
			this.row = null
			this.removed = true
			this.ports.hint(this.key, false)
		})
	}

	/** A caller that handed a value in through `applyExternal` and wrote its own row for it,
	 *  rather than leaving that to this entry: the row is accounted for, and counting it as
	 *  unaccounted would read as someone else having written it. */
	noteRow(): void {
		this.values++
	}

	/** An outside write (the AI chat, another editor): a real divergence, never settling. */
	applyExternal(value: V): number {
		this.externals++
		if (this.loaded && serialize(value) === serialize(this.value)) return this.revision
		this.edits++
		this.pristine = false
		this.removed = false
		this.replaceValue(value)
		// Whatever row this produces is counted by `reconcile`; a value that produces none
		// accounts for nothing, and must not pay for a row some other writer puts there.
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
			if (this.retired) return superseded
			// `canSave` is advisory and not every editor consults it, so the gate is here too:
			// what would go out is a whole value built on a baseline known to be behind.
			if (this.stale) return outOfDate
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
			// Where the write lands, and whether this entry becomes the item there: one that stands
			// for a deployed item writes it and stays where it is.
			const to =
				this.standsFor ?? (adapter.pathOf ?? ((v: V) => (v as { path: string }).path))(sent)
			const elsewhere = to !== from.path
			const moved = elsewhere && this.standsFor === undefined
			const claim = elsewhere ? this.store.claim({ ...from, path: to }, this) : undefined
			let wrote = false
			try {
				await claim?.turn
				if (this.retired) return superseded
				let held: V
				try {
					if (!adapter.write) throw new Error('This item cannot be saved')
					held =
						(await adapter.write({
							...from,
							value: sent,
							deployed: this.origin === 'deployed' ? snapshot(this.deployed) : undefined,
							standsFor: this.standsFor,
							meta: this.meta
						})) ?? sent
				} catch (e) {
					this.error = errorMessage(e)
					return { ok: false, error: this.error }
				}
				wrote = true
				this.error = undefined
				if (held !== sent) this.adopt(sent, held)
				this.serverDeployed = held
				this.origin = 'deployed'
				this.template = undefined
				if (moved) this.moveTo(to)
				this.reconcile()
				await this.settleRows(moved ? [from, this.key] : [this.key])
			} finally {
				// Before telling the key: whoever re-reads it takes its turn there, which this holds.
				claim?.release()
			}
			// The item at `to` has changed under whoever else is showing it.
			if (elsewhere && wrote) await this.store.changed({ ...from, path: to }, this)
			return { ok: true, path: to, moved }
		}
		if (!started) return this.run(body)
		const result = this.queue.then(() => this.turn(body)).finally(started)
		this.queue = result.catch(() => {})
		return result
	}

	/** Give the value each field the server set otherwise than `sent` asked, unless the user has
	 *  changed that field since: left as sent, it would read as a draft of a change nobody made. */
	private adopt(sent: V, held: V): void {
		if (this.value === undefined) return
		const s = sent as Record<string, unknown>
		const h = held as Record<string, unknown>
		const next = snapshot(this.value) as Record<string, unknown>
		let changed = false
		for (const k of new Set([...Object.keys(s), ...Object.keys(h)])) {
			if (deepEqual(s[k], h[k]) || !deepEqual(next[k], s[k])) continue
			next[k] = h[k]
			changed = true
		}
		if (changed) this.replaceValue(next as V)
	}

	/** `asked`: for a caller retrying after recovering the item, the state as of the *first*
	 *  attempt. Taking a fresh one would let an edit made during that recovery look like part of
	 *  what the discard was asked to throw away. */
	discard(asked?: AsOf): Promise<DiscardOutcome> {
		return this.run(async (at) => {
			this.ownsRow = false
			if (!this.loaded || this.retired) return { removed: false }
			// Whatever it decides below, this key's row is this entry's business: it has one, or
			// it is about to keep one. Read here, after any load queued in front of this.
			this.ownsRow = this.row !== null || this.dirty
			// Reverting to a baseline known to be behind the server would write it back.
			if (this.stale) return { removed: false }
			// An edit typed or an outside write landed since the discard was asked for is newer
			// than it, and reverting that would drop it with nothing left holding it. A value a
			// read in front of it installed is not something that moved, it is the read
			// answering, and is what the discard was asked to throw away.
			if (this.edits !== at.edits) return { removed: false }
			if (this.origin === 'draft') {
				const kept = snapshot(this.value)
				const keptRow = this.row
				// `at` is the other window, before this command's turn. This one is its own: taken
				// after the discard's own replacement, which must not count as something moving.
				const editsAtStart = this.edits
				this.replaceValue(undefined)
				this.reconcile()
				await this.settleRows([this.key])
				// An outside write that arrived while the delete was going is newer than this
				// discard: it has already put the item back, and its row is the item.
				if (this.edits !== editsAtStart) return { removed: false }
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
		}, asked)
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
		const id = nextPatch++
		const onTemplate = this.origin === 'new'
		const sent = snapshot(fields) as Record<string, unknown>
		if (onTemplate) {
			// A template is the create's own starting point, not a server row: nothing else is
			// racing it, so the patch lands on it directly.
			if (this.template !== undefined) this.template = { ...this.template, ...sent } as V
		} else {
			for (const [k, v] of Object.entries(sent)) this.patched[k] = { by: id, value: v }
		}
		if (this.value !== undefined) Object.assign(this.value as object, fields)
		this.touch()
		// Taken after this patch's own touch, so it counts only what happens next. The value it
		// put on screen is the user's copy, and a field carrying the value this patch sent is not
		// proof this patch put it there: the chat can write a draft holding the same value.
		const appliedAt = this.asOf()
		/** Release each field this patch still owns; one a later patch has taken stays with it. */
		const release = () => {
			for (const k of Object.keys(sent)) {
				if (this.patched[k]?.by === id) delete this.patched[k]
			}
		}
		return this.run(async () => {
			if (this.retired) return superseded
			let failed: string | undefined
			try {
				await write(this.key)
			} catch (e) {
				failed = errorMessage(e)
			}
			// The server took it: it holds this now, under whatever a later patch is still ahead
			// of it on. Refused: dropping the overlay is the whole rollback of the baseline.
			if (!failed && this.serverDeployed !== undefined && !onTemplate) {
				this.serverDeployed = { ...this.serverDeployed, ...sent } as V
			}
			release()
			// The value is the user's copy, given back by hand: a field still carrying what this
			// patch put there goes to the baseline as it stands now, so an edit made meanwhile to
			// another field stays. None of them once an outside write has landed — that wrote the
			// value whole and is newer, and a field of it matching is not this patch's to reclaim.
			if (failed) {
				const base = (onTemplate ? this.template : this.deployed) as
					| Record<string, unknown>
					| undefined
				const written = this.externals !== appliedAt.externals
				if (base !== undefined && this.value !== undefined && !written) {
					const out = snapshot(this.value) as Record<string, unknown>
					let changed = false
					for (const [k, v] of Object.entries(sent)) {
						if (!deepEqual(out[k], v)) continue
						out[k] = base[k]
						changed = true
					}
					if (changed) this.replaceValue(out as V)
				}
				this.error = failed
				this.reconcile()
				return { ok: false, error: failed }
			}
			this.error = undefined
			this.touch()
			this.reconcile()
			return { ok: true }
		})
	}

	/** Fetch something about the deployed item the load did not carry (a secret's plaintext),
	 *  and write it into both sides. */
	learn(fetch: (key: ItemKey) => Promise<(side: V) => void>): Promise<CommandOutcome> {
		return this.run(async (at) => {
			let apply: (side: V) => void
			try {
				apply = await fetch(this.key)
			} catch (e) {
				return { ok: false, error: errorMessage(e) }
			}
			if (this.deployed !== undefined) {
				const d = snapshot(this.deployed)
				apply(d)
				this.serverDeployed = d
			}
			// What it learned is about the deployed item, so that side takes it either way. The
			// value is the user's, and anything written there while the fetch was out is newer
			// than this: folding a decrypted secret from before into it would put the old one
			// back and autosave it.
			const moved = this.edits !== at.edits || this.externals !== at.externals
			if (this.value !== undefined && !moved) {
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
			if (this.retired) return superseded
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
	claim(key: ItemKey, claimant: Entry<any>): { turn: Promise<unknown>; release: () => void }
	moveSettled(entry: Entry<any>): Promise<void>
	changed(key: ItemKey, by: Entry<any>): Promise<void>
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
	/** `dirty && valid && writable && !busy`, and not stale: a write landed on this item
	 *  elsewhere and the re-read meant to pick it up failed, so saving would put a baseline known
	 *  to be behind the server back over it. A reload clears that. */
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
	/**
	 * This item's value describes the deployed item at this path, from a config its caller holds
	 * (a runnable's trigger panel): saving writes that item without becoming it. Kept under its own
	 * temporary key, so the caller's draft stays the caller's and one entry owns the item's row.
	 */
	standsFor?: string
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
			!this.entry.busy &&
			!this.entry.stale
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
	/** The latest save moving onto a key, by that key, and when its move is done. */
	const moves = new Map<string, { owner: Entry<any>; done: Promise<void> }>()
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
		},
		/** A save about to write the item at `key`. It goes after the command running there and the
		 *  move already heading there, and every other command there waits for it in turn
		 *  (`moveSettled`), so one write lands at a time and what it retires is idle by then. Never
		 *  behind an entry that is waiting for this one: each would wait for the other. */
		claim(key, claimant) {
			const k = keyString(key)
			const holder = entries.get(k)
			const previous = moves.get(k)
			let release!: () => void
			const move = { owner: claimant, done: new Promise<void>((r) => (release = r)) }
			moves.set(k, move)
			const waits: Promise<unknown>[] = []
			claimant.waitingOn = []
			if (previous && !waitsFor(previous.owner, claimant)) {
				waits.push(previous.done)
				claimant.waitingOn.push(previous.owner)
			}
			if (holder?.running && !waitsFor(holder, claimant)) {
				waits.push(holder.running.catch(() => {}))
				claimant.waitingOn.push(holder)
			}
			return {
				turn: Promise.all(waits).then(() => (claimant.waitingOn = [])),
				release() {
					release()
					if (moves.get(k) === move) moves.delete(k)
				}
			}
		},
		/**
		 * The item at `key` was written by another entry. Whoever holds it re-reads it: the edits on
		 * screen there are in its draft row, so they come back over what was just written. With
		 * nobody there, the row that is left predates the write and goes.
		 */
		async changed(key, by) {
			const holder = entries.get(keyString(key))
			if (holder === by) return
			// Its row is debounced, so `reread` sends what is queued before reading it back.
			if (holder) {
				// Failing here does not fail the write, which landed: it leaves whoever holds the
				// item on a baseline older than the server, and unable to act until it reloads.
				if (!(await holder.reread()).ok) holder.stale = true
				return
			}
			ports.write(key, null)
			await ports.flush(key)
		},
		async moveSettled(entry) {
			let move = moves.get(keyString(entry.key))
			while (move && move.owner !== entry) {
				await move.done
				move = moves.get(keyString(entry.key))
			}
		}
	}

	function waitsFor(from: Entry<any>, target: Entry<any>): boolean {
		const seen = new Set<Entry<any>>()
		const pending = [from]
		for (let e = pending.pop(); e; e = pending.pop()) {
			if (e === target) return true
			if (seen.has(e)) continue
			seen.add(e)
			pending.push(...e.waitingOn)
		}
		return false
	}

	/**
	 * An entry moved onto a key another live entry holds: a create or a rename onto the path a
	 * draft-only item occupies, which the write supersedes. One key has one row writer, so the old
	 * one stops writing and every handle on it moves to the entry that now is the item.
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
			entry.adapter = adapter
			entry.standsFor = spec.standsFor
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
			if (value === undefined || value === null) return false
			const entry = find(workspace, kind, path)
			// With nobody holding the key, the caller persists it as a row; an entry arriving there
			// reads it like any other draft.
			if (!entry) return false
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
		refresh(
			workspace: string,
			kind: UserDraftItemKind,
			path: string
		): Promise<'done' | 'absent' | 'failed'> | undefined {
			return find(workspace, kind, path)?.refreshed()
		},
		noteRow(workspace: string, kind: UserDraftItemKind, path: string): void {
			find(workspace, kind, path)?.noteRow()
		},
		itemDeleted(workspace: string, kind: UserDraftItemKind, path: string): Promise<void> {
			return find(workspace, kind, path)?.itemDeleted() ?? Promise.resolve()
		},
		discard(
			workspace: string,
			kind: UserDraftItemKind,
			path: string
		): Promise<'done' | 'absent' | 'failed'> | undefined {
			const entry = find(workspace, kind, path)
			if (!entry) return undefined
			// Returned rather than left running: the caller would otherwise send its own delete
			// alongside this one, and the second goes out with no baseline to check. It resolves
			// false if this entry turns out not to have dealt with the row, so the caller still can.
			return entry.discarded()
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
	rowMark(key) {
		return UserDraftDbSyncer.sendsSoFar(keyQuery(key))
	},
	pending(key) {
		return UserDraftDbSyncer.pendingValue(keyQuery(key))
	},
	unbased(key) {
		return UserDraftDbSyncer.hasUnbasedPending(keyQuery(key))
	},
	markConflict(key, serverTimestamp) {
		UserDraftDbSyncer.markConflict(keyQuery(key), serverTimestamp)
	},
	seedSync(key, draftSavedAt, since) {
		UserDraftDbSyncer.recordRemoteSync(keyQuery(key), draftSavedAt, since)
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
	const sessionOf = new Map<ItemAcquisition<V>, string>()
	const idOf = (key: ItemKey, session: unknown) => `${keyString(key)}#${String(session ?? '')}`

	function reconcile() {
		const specs = getSpecs()
		const wanted: { id: string; key: ItemKey; spec: ItemSpec<V> }[] = []
		for (const spec of specs) {
			if (!spec.workspace || !spec.path) {
				wanted.push({ id: '', key: undefined as unknown as ItemKey, spec })
				continue
			}
			const key: ItemKey = { workspace: spec.workspace, kind, path: spec.path }
			wanted.push({ id: idOf(key, spec.session), key, spec })
		}
		const ids = new Set(wanted.map((w) => w.id))
		// A spec that caught up with where a save moved its item keeps that item, rather than
		// releasing it and reading it afresh over edits whose row may not have landed yet.
		for (const { id } of wanted) {
			if (!id || held.has(id)) continue
			for (const [heldId, acq] of held) {
				if (ids.has(heldId) || idOf(acq.handle.key, sessionOf.get(acq)) !== id) continue
				held.delete(heldId)
				held.set(id, acq)
				break
			}
		}
		// Release first, so an item reopened under a new session is read afresh when idle.
		for (const [id, acq] of [...held]) {
			if (!ids.has(id)) {
				acq.release()
				held.delete(id)
				sessionOf.delete(acq)
			}
		}
		const next = wanted.map(({ id, key, spec }) => {
			if (!id) return undefined
			let acq = held.get(id)
			if (!acq) {
				acq = itemStore.acquire(key, spec, adapter)
				held.set(id, acq)
				sessionOf.set(acq, String(spec.session ?? ''))
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
		sessionOf.clear()
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
