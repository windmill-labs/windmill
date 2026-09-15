import { describe, it, expect } from 'vitest'
import {
	createItemStore,
	newItemPath,
	type ItemAdapter,
	type ItemKey,
	type ItemLoad,
	type ItemRowPort,
	type ItemSpec,
	type ItemWriteContext
} from './itemStore.svelte'

type Res = { path: string; description: string; args: Record<string, unknown> }

function deferred<T = void>() {
	let resolve!: (v: T) => void
	let reject!: (e: unknown) => void
	const promise = new Promise<T>((res, rej) => {
		resolve = res
		reject = rej
	})
	return { promise, resolve, reject }
}

/** Records every row write, the way the syncer would receive them. */
function fakeRows() {
	const writes: { path: string; value: unknown }[] = []
	const conflicts = new Set<string>()
	/** Paths whose next write the server fails. */
	const failing = new Set<string>()
	/** Paths the server rejects whenever a row is actually sent, until a read adopts a fresh
	 *  baseline — which is what makes a payload it refused acceptable again. */
	const conflictOnFlush = new Set<string>()
	const failures = new Map<string, string>()
	const hints = new Map<string, boolean>()
	/** Paths whose parked payload was dropped. */
	const dropped: string[] = []
	/** Every `last_sync` baseline the syncer accepted, in order. */
	const seeds: { path: string; at: string | undefined }[] = []
	/** Rows handed over per path, by any writer — the store's reconcile or the chat's own save. */
	const sends = new Map<string, number>()
	const handed = (path: string) => sends.set(path, (sends.get(path) ?? 0) + 1)
	/** What a write leaves queued, and what a flush sends: the syncer debounces. */
	const queued = new Map<string, unknown>()
	/** A payload the server refused, kept for a later flush the way the syncer parks one. */
	const parked = new Map<string, unknown>()
	const sent = new Map<string, unknown>()
	/** Set by a test to keep a flush in flight. */
	let holdFlush: Promise<void> | undefined
	const port: ItemRowPort = {
		write: (key, value) => {
			writes.push({ path: key.path, value })
			queued.set(key.path, value)
			handed(key.path)
			if (failing.has(key.path)) failures.set(key.path, 'unreachable')
		},
		flush: async (key) => {
			// A POST carries what was queued when it went out; what is typed while it is in flight
			// queues behind it. With nothing queued, a refused payload is replayed instead.
			const fromQueue = queued.has(key.path)
			const going = fromQueue
				? { value: queued.get(key.path) }
				: parked.has(key.path)
					? { value: parked.get(key.path) }
					: undefined
			queued.delete(key.path)
			if (holdFlush) await holdFlush
			if (!going) return
			// Replaying a parked payload is another POST, so another hand-over.
			if (!fromQueue) handed(key.path)
			// A POST that never reaches the server keeps its payload for the next attempt.
			if (failing.has(key.path)) {
				failures.set(key.path, 'unreachable')
				parked.set(key.path, going.value)
				return
			}
			if (conflictOnFlush.has(key.path)) {
				conflicts.add(key.path)
				parked.set(key.path, going.value)
				return
			}
			parked.delete(key.path)
			sent.set(key.path, going.value)
		},
		overwrite: async (key, value) => {
			conflicts.delete(key.path)
			writes.push({ path: key.path, value })
			handed(key.path)
		},
		rowMark: (key) => sends.get(key.path) ?? 0,
		seedSync: (key, at, since) => {
			// The syncer refuses a baseline from a read a row handed over since has passed.
			if (since !== (sends.get(key.path) ?? 0)) return
			seeds.push({ path: key.path, at })
			// Back in sync with the server, as `recordRemoteSync` does. The baseline is fresh, so
			// a payload the server was refusing would be accepted from here on.
			conflicts.delete(key.path)
			conflictOnFlush.delete(key.path)
			failures.delete(key.path)
		},
		conflicted: (key) => conflicts.has(key.path),
		failure: (key) => failures.get(key.path),
		dropPending: (key) => {
			dropped.push(key.path)
			parked.delete(key.path)
		},
		hint: (key, on) => void hints.set(key.path, on)
	}
	return {
		port,
		writes,
		conflicts,
		conflictOnFlush,
		failing,
		hints,
		dropped,
		seeds,
		sent,
		/** A row handed over by someone other than the store, as the chat's own save does. */
		handExternally: handed,
		hold: (until: Promise<void>) => void (holdFlush = until)
	}
}

const deployedRes: Res = { path: 'u/me/r', description: 'deployed', args: { a: 1 } }

function adapter(
	load: ItemLoad<Res> | (() => Promise<ItemLoad<Res>>),
	write: (ctx: ItemWriteContext<Res>) => Promise<void> = async () => {}
): ItemAdapter<Res> & { writes: ItemWriteContext<Res>[] } {
	const writes: ItemWriteContext<Res>[] = []
	return {
		writes,
		load: typeof load === 'function' ? load : async () => structuredClone(load),
		write: async (ctx) => {
			writes.push(ctx)
			await write(ctx)
		}
	}
}

async function open(
	rows: ReturnType<typeof fakeRows>,
	a: ItemAdapter<Res>,
	path = 'u/me/r',
	spec: Partial<ItemSpec<Res>> = {}
) {
	const store = createItemStore(rows.port)
	const key: ItemKey = { workspace: 'w', kind: 'resource', path }
	const acq = store.acquire(key, { workspace: 'w', path, ...spec }, a)
	await settle()
	return { store, item: acq.handle, release: acq.release }
}

/** Let queued commands run. */
const settle = () => new Promise((r) => setTimeout(r, 0))

describe('item store: the draft row follows value ≠ deployed', () => {
	it('writes a row when the value diverges and deletes it when it comes back', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }))
		expect(item.dirty).toBe(false)
		expect(rows.writes).toEqual([])

		item.value = { ...deployedRes, description: 'edited' }
		expect(item.dirty).toBe(true)
		expect(rows.writes).toEqual([
			{ path: 'u/me/r', value: { ...deployedRes, description: 'edited' } }
		])
		expect(rows.hints.get('u/me/r')).toBe(true)

		item.value = structuredClone(deployedRes)
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
		expect(rows.hints.get('u/me/r')).toBe(false)
	})

	it('deletes the row after a save without anyone asking for it', async () => {
		const rows = fakeRows()
		const a = adapter({ deployed: deployedRes })
		const { item } = await open(rows, a)
		item.value = { ...deployedRes, description: 'edited' }

		const outcome = await item.save()

		expect(outcome).toEqual({ ok: true, path: 'u/me/r', moved: false })
		expect(a.writes[0].deployed).toEqual(deployedRes)
		expect(item.deployed).toEqual({ ...deployedRes, description: 'edited' })
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
	})

	it('removes a stale row that equals the deployed value on load', async () => {
		const rows = fakeRows()
		await open(rows, adapter({ deployed: deployedRes, draft: structuredClone(deployedRes) }))
		expect(rows.writes).toEqual([{ path: 'u/me/r', value: null }])
	})
})

describe('item store: commands', () => {
	it('keeps an edit typed while a save is in flight, and its row', async () => {
		const rows = fakeRows()
		const gate = deferred()
		const { item } = await open(
			rows,
			adapter({ deployed: deployedRes }, () => gate.promise)
		)
		item.value = { ...deployedRes, description: 'sent' }

		const saving = item.save()
		expect(item.status).toBe('saving')
		item.value = { ...deployedRes, description: 'typed during the save' }
		gate.resolve()
		expect(await saving).toMatchObject({ ok: true })

		expect(item.deployed?.description).toBe('sent')
		expect(item.value?.description).toBe('typed during the save')
		expect(item.dirty).toBe(true)
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed during the save' }
		})
	})

	it('runs commands on one item one at a time, and a second save of the same value writes nothing', async () => {
		const rows = fakeRows()
		const gate = deferred()
		const a = adapter({ deployed: deployedRes }, () => gate.promise)
		const { item } = await open(rows, a)
		item.value = { ...deployedRes, description: 'edited' }

		const first = item.save()
		const second = item.save()
		await settle()
		expect(a.writes).toHaveLength(1)
		expect(item.canSave).toBe(false)

		gate.resolve()
		expect(await first).toMatchObject({ ok: true })
		expect(await second).toMatchObject({ ok: true })
		expect(a.writes).toHaveLength(1)
	})

	it('saves items in turn, all busy from the call, and stops at the first failure', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const written: string[] = []
		const a = adapter({ deployed: deployedRes }, async (ctx) => {
			written.push(ctx.workspace)
			if (ctx.workspace === 'w1') throw new Error('refused')
		})
		const acquire = (ws: string) =>
			store.acquire(
				{ workspace: ws, kind: 'resource', path: 'u/me/r' },
				{ workspace: ws, path: 'u/me/r' },
				a
			).handle
		const [first, second] = [acquire('w1'), acquire('w2')]
		await settle()
		first.value = { ...deployedRes, description: 'one' }
		second.value = { ...deployedRes, description: 'two' }

		const saving = store.saveEach([first, second])
		expect([first.busy, second.busy]).toEqual([true, true])
		const outcomes = await saving

		expect(outcomes).toEqual([
			{ ok: false, error: 'refused' },
			{ ok: false, error: 'Not saved', skipped: true }
		])
		expect(written).toEqual(['w1'])
		expect([first.dirty, second.dirty, second.busy]).toEqual([true, true, false])
	})

	it('keeps a toggle made during a save out of the draft row', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const saveGate = deferred()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ deployed: { path: 's', enabled: true, summary: 'a' } }),
				write: () => saveGate.promise
			} as ItemAdapter<Sched>
		)
		await settle()
		item.value = { path: 's', enabled: true, summary: 'b' }
		const saving = item.save()
		const toggling = item.patch({ enabled: false }, async () => {})
		saveGate.resolve()
		await saving
		// The save's baseline predates the toggle; the toggle's field survives it, so nothing
		// is left unsaved and the row goes.
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 's', value: null })
		await toggling
		expect(item.deployed).toEqual({ path: 's', enabled: false, summary: 'b' })
		expect(rows.writes.at(-1)).toEqual({ path: 's', value: null })
	})

	it('lets the later of two overlapping toggles stand, without a row in between', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const first = deferred()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ deployed: { path: 's', enabled: false, summary: 'a' } })
			} as ItemAdapter<Sched>
		)
		await settle()
		const enabling = item.patch({ enabled: true }, () => first.promise)
		const disabling = item.patch({ enabled: false }, async () => {})
		first.resolve()

		expect(await enabling).toEqual({ ok: true })
		// The first request landing must not put its own `enabled` back over the second's.
		expect(item.dirty).toBe(false)
		expect(await disabling).toEqual({ ok: true })
		expect(item.deployed).toEqual({ path: 's', enabled: false, summary: 'a' })
		expect(item.dirty).toBe(false)
		expect(rows.writes).toEqual([])
	})

	it('keeps a toggle made during a read the server answered before it', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const reading = deferred()
		let reads = 0
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => {
					// The read in flight predates the toggle, so it still reports it disabled.
					if (reads++ > 0) await reading.promise
					return { deployed: { path: 's', enabled: false, summary: 'a' } }
				}
			} as ItemAdapter<Sched>
		)
		await settle()
		const reloading = item.reload()
		await settle()
		const toggling = item.patch({ enabled: true }, async () => {})
		reading.resolve()

		expect(await reloading).toEqual({ ok: true })
		// The read must not put the schedule back to disabled under the toggle: that reads as an
		// unsaved change to `enabled` and would be written as a draft.
		expect(item.dirty).toBe(false)
		expect(await toggling).toEqual({ ok: true })
		expect(item.deployed).toEqual({ path: 's', enabled: true, summary: 'a' })
		expect(rows.writes).toEqual([])
	})

	it('holds a toggle behind a save, and keeps an unrelated edit through the toggle', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const saveGate = deferred()
		const order: string[] = []
		const store = createItemStore(rows.port)
		const a: ItemAdapter<Sched> = {
			load: async () => ({ deployed: { path: 's', enabled: true, summary: 'a' } }),
			write: async () => {
				order.push('save:start')
				await saveGate.promise
				order.push('save:end')
			}
		}
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			a
		)
		await settle()
		item.value = { path: 's', enabled: true, summary: 'b' }
		const saving = item.save()
		item.value = { path: 's', enabled: true, summary: 'unrelated edit' }
		const toggling = item.patch({ enabled: false }, async () => void order.push('toggle'))
		saveGate.resolve()
		await Promise.all([saving, toggling])

		expect(order).toEqual(['save:start', 'save:end', 'toggle'])
		expect(item.deployed).toEqual({ path: 's', enabled: false, summary: 'b' })
		expect(item.value).toEqual({ path: 's', enabled: false, summary: 'unrelated edit' })
		expect(rows.writes.at(-1)?.value).toEqual({
			path: 's',
			enabled: false,
			summary: 'unrelated edit'
		})
	})

	it('gives a failed toggle its field back', async () => {
		type Sched = { path: string; enabled: boolean }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{ load: async () => ({ deployed: { path: 's', enabled: true } }) } as ItemAdapter<Sched>
		)
		await settle()
		const refusal = deferred()
		const toggling = item.patch({ enabled: false }, () => refusal.promise)
		// In flight the toggle is the server's to hold, not a draft of the user's.
		expect(item.dirty).toBe(false)
		expect(rows.writes).toEqual([])

		refusal.reject(new Error('refused'))
		expect(await toggling).toEqual({ ok: false, error: 'refused' })
		expect(item.value).toEqual({ path: 's', enabled: true })
		expect(item.deployed).toEqual({ path: 's', enabled: true })
		expect(rows.writes).toEqual([])
	})

	it('gives the server value back when two overlapping toggles are both refused', async () => {
		type Sched = { path: string; enabled: boolean }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{ load: async () => ({ deployed: { path: 's', enabled: false } }) } as ItemAdapter<Sched>
		)
		await settle()
		const first = deferred()
		const second = deferred()
		const enabling = item.patch({ enabled: true }, () => first.promise)
		const disabling = item.patch({ enabled: false }, () => second.promise)
		first.reject(new Error('refused'))
		second.reject(new Error('refused'))

		expect(await enabling).toMatchObject({ ok: false })
		expect(await disabling).toMatchObject({ ok: false })
		// The second's rollback target is what the server holds, not what the first optimistically
		// put there and never got accepted.
		expect(item.deployed).toEqual({ path: 's', enabled: false })
		expect(item.value).toEqual({ path: 's', enabled: false })
		expect(rows.writes).toEqual([])
	})

	it('keeps a toggle whose value an earlier accepted one repeats', async () => {
		type Sched = { path: string; enabled: boolean }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{ load: async () => ({ deployed: { path: 's', enabled: false } }) } as ItemAdapter<Sched>
		)
		await settle()
		const gates = [deferred(), deferred(), deferred()]
		// enable, disable, enable: the first and third carry the same value, so only a token can
		// say which of them still owns the field.
		const first = item.patch({ enabled: true }, () => gates[0].promise)
		const second = item.patch({ enabled: false }, () => gates[1].promise)
		const third = item.patch({ enabled: true }, () => gates[2].promise)
		gates[0].resolve()
		gates[1].reject(new Error('refused'))
		gates[2].reject(new Error('refused'))

		expect(await first).toEqual({ ok: true })
		expect(await second).toMatchObject({ ok: false })
		expect(await third).toMatchObject({ ok: false })
		// The server took the first and refused the rest, so it holds `true`.
		expect(item.deployed).toEqual({ path: 's', enabled: true })
		expect(item.value).toEqual({ path: 's', enabled: true })
		expect(rows.writes).toEqual([])
	})

	it('saves a change the draft comparison ignores', async () => {
		type Sched = { path: string; permissioned_as?: string }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const writes: unknown[] = []
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ deployed: { path: 's', permissioned_as: 'u/a' } }),
				write: async (ctx) => void writes.push(ctx.value)
			} as ItemAdapter<Sched>
		)
		await settle()
		item.value = { path: 's', permissioned_as: 'u/b' }
		expect(item.dirty).toBe(false)
		expect(await item.save()).toMatchObject({ ok: true })
		expect(writes).toEqual([{ path: 's', permissioned_as: 'u/b' }])
	})

	it('records what a write left on the server, and gives the value the fields it set', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const gate = deferred()
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ draft: { path: 's', enabled: false, summary: '' } }),
				write: async (ctx) => {
					await gate.promise
					return { ...ctx.value, enabled: true }
				}
			} as ItemAdapter<Sched>
		)
		await settle()

		const saving = item.save()
		item.value = { path: 's', enabled: false, summary: 'typed during the save' }
		gate.resolve()
		expect(await saving).toMatchObject({ ok: true })

		const kept = { path: 's', enabled: true, summary: 'typed during the save' }
		expect(item.deployed).toEqual({ path: 's', enabled: true, summary: '' })
		expect(item.value).toEqual(kept)
		expect(rows.writes.at(-1)).toEqual({ path: 's', value: kept })
	})
})

describe('item store: outside writes', () => {
	it('does not let a read that started earlier replace a newer outside write', async () => {
		const rows = fakeRows()
		let reads = 0
		const second = deferred<ItemLoad<Res>>()
		const a = adapter(async () => (reads++ === 0 ? { deployed: deployedRes } : second.promise))
		const { item } = await open(rows, a)

		const reloading = item.reload()
		await settle()
		item.applyExternal({ ...deployedRes, description: 'from the chat' })
		second.resolve({ deployed: { ...deployedRes, description: 'deployed elsewhere' } })
		await reloading

		expect(item.value?.description).toBe('from the chat')
		expect(item.deployed?.description).toBe('deployed elsewhere')
		expect(item.dirty).toBe(true)
	})

	it('keeps an outside write that lands before the first read does', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		item.applyExternal({ ...deployedRes, description: 'from the chat' })
		read.resolve({ deployed: deployedRes })
		await settle()

		expect(item.value?.description).toBe('from the chat')
		expect(item.deployed).toEqual(deployedRes)
		expect(rows.writes).toEqual([
			{ path: 'u/me/r', value: { ...deployedRes, description: 'from the chat' } }
		])
	})

	it('does not rewind the row baseline to what a read that started earlier saw', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const { item } = await open(
			rows,
			adapter(() => {
				reads++
				return reads > 1
					? read.promise
					: Promise.resolve({ deployed: deployedRes, draftSavedAt: 'T1' })
			})
		)
		expect(rows.seeds).toEqual([{ path: 'u/me/r', at: 'T1' }])

		const reloading = item.reload()
		await settle()
		// The chat writes while the read is out; its row goes, and the syncer moves past T1.
		item.applyExternal({ ...deployedRes, description: 'from the chat' })
		read.resolve({ deployed: deployedRes, draft: deployedRes, draftSavedAt: 'T1' })
		await reloading

		expect(item.value?.description).toBe('from the chat')
		// Reseeding T1 here would put the syncer behind the row it has already sent, and the
		// server refuses everything after that as a conflict.
		expect(rows.seeds).toEqual([{ path: 'u/me/r', at: 'T1' }])
		expect(item.status).not.toBe('conflicted')
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'from the chat' }
		})
	})

	it('applies outside writes in order, each one a new revision and never settling', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }), 'u/me/r', {})
		const r1 = item.applyExternal({ ...deployedRes, description: 'one' })
		const r2 = item.applyExternal({ ...deployedRes, description: 'two' })
		expect(r2).toBeGreaterThan(r1)
		expect(item.value?.description).toBe('two')
		expect(rows.writes.at(-1)?.value).toMatchObject({ description: 'two' })
	})
})

describe('item store: origins', () => {
	it('discards a deployed item back to its deployed value', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }))
		item.value = { ...deployedRes, description: 'edited' }
		expect(await item.discard()).toEqual({ removed: false })
		expect(item.value).toEqual(deployedRes)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
	})

	it('removes a draft-only item on discard', async () => {
		const rows = fakeRows()
		const draft = { ...deployedRes, description: 'only a draft' }
		const { item } = await open(rows, adapter({ draft }))
		expect(item.origin).toBe('draft')
		expect(item.dirty).toBe(true)
		expect(rows.writes).toEqual([])

		expect(await item.discard()).toEqual({ removed: true })
		expect(item.removed).toBe(true)
		expect(item.value).toBeUndefined()
		expect(rows.writes).toEqual([{ path: 'u/me/r', value: null }])
	})

	it('keeps an edit typed after a discard was asked for, while it waited its turn', async () => {
		const rows = fakeRows()
		const gate = deferred()
		const { item } = await open(
			rows,
			adapter({ deployed: deployedRes }, () => gate.promise)
		)
		item.value = { ...deployedRes, description: 'edited' }
		const saving = item.save()
		await settle()
		const discarding = item.discard()
		await settle()
		item.value = { ...deployedRes, description: 'typed after the discard' }
		gate.resolve()

		expect(await saving).toMatchObject({ ok: true })
		// The discard is older than what is on screen: reverting to `deployed` would drop it.
		expect(await discarding).toEqual({ removed: false })
		expect(item.value).toEqual({ ...deployedRes, description: 'typed after the discard' })
		expect(item.dirty).toBe(true)
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed after the discard' }
		})
	})

	it('discards behind an outside write of the value already on screen', async () => {
		const rows = fakeRows()
		const gate = deferred()
		const { item } = await open(
			rows,
			adapter({ deployed: deployedRes }, () => gate.promise)
		)
		item.value = { ...deployedRes, description: 'edited' }
		const saving = item.save()
		await settle()
		const discarding = item.discard()
		await settle()
		// The chat saves what the user is already looking at: nothing on screen moves, so the
		// discard is still about the value its click named.
		item.applyExternal({ ...deployedRes, description: 'edited' })
		gate.resolve()

		expect(await saving).toMatchObject({ ok: true })
		expect(await discarding).toEqual({ removed: false })
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
	})

	it('discards the draft a read turns up when the discard was asked for before it', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		// The chat discards the draft while this editor's read is still out: nothing is on screen
		// yet, so there is no value the discard could be measured against.
		const discarding = item.discard()
		read.resolve({ draft: { ...deployedRes, description: 'only a draft' } })

		expect(await discarding).toEqual({ removed: true })
		expect(item.removed).toBe(true)
		expect(item.value).toBeUndefined()
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
	})

	it('keeps an outside write that landed after a discard was asked for, before the first read', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		const discarding = item.discard()
		// The chat writes after asking for the discard, so its value is the newer of the two.
		const fromChat = { ...deployedRes, description: 'from the chat' }
		item.applyExternal(fromChat)
		read.resolve({ deployed: deployedRes })

		expect(await discarding).toEqual({ removed: false })
		expect(item.value).toEqual(fromChat)
		expect(item.dirty).toBe(true)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: fromChat })
	})

	it('keeps a draft-only item an outside write put back while its delete was going', async () => {
		const rows = fakeRows()
		const draft = { ...deployedRes, description: 'only a draft' }
		const { item } = await open(rows, adapter({ draft }))
		const gate = deferred()
		rows.hold(gate.promise)
		const discarded = item.discard()
		await settle()
		// The chat writes the same item while the delete is in flight.
		const fromChat = { ...draft, description: 'written by the chat' }
		item.applyExternal(fromChat)
		gate.resolve()

		expect(await discarded).toEqual({ removed: false })
		expect(item.removed).toBe(false)
		expect(item.value).toEqual(fromChat)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: fromChat })
	})

	it('keeps a draft-only item whose delete did not land', async () => {
		const rows = fakeRows()
		const draft = { ...deployedRes, description: 'only a draft' }
		const { item } = await open(rows, adapter({ draft }))
		rows.failing.add('u/me/r')

		expect(await item.discard()).toEqual({ removed: false })
		expect(item.removed).toBe(false)
		expect(item.value).toEqual(draft)
		expect(item.status).toBe('failed')
		expect(rows.hints.get('u/me/r')).toBe(true)
		expect(rows.dropped).toContain('u/me/r')
	})

	it('creates under a temporary path, then moves to the real one and clears any row there', async () => {
		const rows = fakeRows()
		const temp = newItemPath()
		const a = adapter({})
		const template: Res = { path: '', description: '', args: {} }
		const { item, store } = await open(rows, a, temp, { template })
		expect(item.origin).toBe('new')
		expect(item.dirty).toBe(false)

		item.value = { path: 'u/me/created', description: 'new', args: { a: 2 } }
		const outcome = await item.save()

		expect(outcome).toEqual({ ok: true, path: 'u/me/created', moved: true })
		expect(a.writes[0]).toMatchObject({ path: temp, deployed: undefined })
		expect(item.key.path).toBe('u/me/created')
		expect(item.origin).toBe('deployed')
		expect(item.dirty).toBe(false)
		// Nothing under the temporary path; a draft left at the real one predates the create.
		expect(rows.writes).toEqual([{ path: 'u/me/created', value: null }])
		expect(store.bridge.read('w', 'resource', 'u/me/created')).toEqual({ value: undefined })
		expect(store.bridge.read('w', 'resource', temp)).toBeUndefined()
	})

	it('moves a renamed item and clears the rows at both paths', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }))
		item.value = { ...deployedRes, path: 'u/me/renamed' }
		expect(rows.writes).toEqual([
			{ path: 'u/me/r', value: { ...deployedRes, path: 'u/me/renamed' } }
		])

		expect(await item.save()).toEqual({ ok: true, path: 'u/me/renamed', moved: true })
		expect(rows.writes.slice(1)).toEqual([
			{ path: 'u/me/r', value: null },
			{ path: 'u/me/renamed', value: null }
		])
	})
})

describe('item store: one entry per key', () => {
	it('moves onto a key another editor holds, which then follows the item there', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const at = (path: string, load: ItemLoad<Res>) =>
			store.acquire(
				{ workspace: 'w', kind: 'resource', path },
				{ workspace: 'w', path },
				adapter(load)
			).handle
		const other = at('u/me/b', {
			draft: { ...deployedRes, path: 'u/me/b', description: 'draft b' }
		})
		const moving = at('u/me/r', { deployed: deployedRes })
		await settle()

		moving.value = { ...deployedRes, path: 'u/me/b' }
		expect(await moving.save()).toMatchObject({ ok: true, moved: true })

		expect(other.key.path).toBe('u/me/b')
		expect(other.value).toEqual({ ...deployedRes, path: 'u/me/b' })
		expect(other.origin).toBe('deployed')
		// The displaced entry writes nothing more: `u/me/b` has one writer.
		const before = rows.writes.length
		other.value = { ...deployedRes, path: 'u/me/b', description: 'typed in the other editor' }
		expect(moving.value?.description).toBe('typed in the other editor')
		expect(rows.writes.slice(before)).toEqual([
			{
				path: 'u/me/b',
				value: { ...deployedRes, path: 'u/me/b', description: 'typed in the other editor' }
			}
		])
	})

	it('writes the item it stands for without becoming it', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		const temporary = newItemPath()
		const a = adapter({})
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			a
		)
		await settle()
		panel.value = { ...b, description: 'from the panel' }

		expect(await panel.save()).toMatchObject({ ok: true, path: 'u/me/b', moved: false })
		expect(a.writes[0].standsFor).toBe('u/me/b')
		// Still its own item under its own key, and a temporary key never holds a row: the config
		// it was opened on stays its caller's draft, not the item's. The only row it touches is the
		// one left at the item it wrote, which predates that write.
		expect(panel.key.path).toBe(temporary)
		expect(rows.writes).toEqual([{ path: 'u/me/b', value: null }])
	})

	it('has whoever holds an item re-read it when someone else writes it', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		// What the server holds: the deployed value, and the row the syncer has actually sent. The
		// editor's edit is only queued, so the re-read has to flush it before reading.
		let deployed = b
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => ({ deployed, draft: rows.sent.get('u/me/b') as Res | undefined }))
		)
		await settle()
		const typed = { ...b, description: 'typed in the open editor' }
		open.value = typed

		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => void (deployed = { ...ctx.value }))
		)
		await settle()
		panel.value = { ...b, args: { a: 2 } }
		expect(await panel.save()).toMatchObject({ ok: true, path: 'u/me/b' })

		// The write landed under it, and its own edit came back over it from its row.
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
		expect(open.value).toEqual(typed)
		expect(open.dirty).toBe(true)
	})

	it('leaves an item discarded while a write was landing on the deployed value', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		let deployed = b
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => ({ deployed, draft: rows.sent.get('u/me/b') as Res | undefined }))
		)
		const gate = deferred()
		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => {
				await gate.promise
				deployed = { ...ctx.value }
			})
		)
		await settle()
		open.value = { ...b, description: 'typed in the open editor' }
		panel.value = { ...b, args: { a: 2 } }
		const wrote = panel.save()
		await settle()
		// Discard while the write is in flight: it queues behind the writer's claim on the key.
		const discarded = open.discard()
		gate.resolve()

		expect(await wrote).toMatchObject({ ok: true, path: 'u/me/b' })
		expect(await discarded).toEqual({ removed: false })
		// The discard holds: what it dropped does not come back as a draft over the new value.
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
		expect(open.value).toEqual({ ...b, args: { a: 2 } })
		expect(open.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/b', value: null })
	})

	it('shows the draft of an editor that closed before its row was sent', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const a = adapter(async () => ({
			deployed: deployedRes,
			draft: rows.sent.get('u/me/r') as Res | undefined
		}))
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		first.handle.value = { ...deployedRes, description: 'typed then closed' }
		// The row is queued, not sent: the syncer debounces it.
		expect(rows.sent.has('u/me/r')).toBe(false)
		first.release()

		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// Reopening must not read past the row the closing editor left queued.
		expect(second.handle.value?.description).toBe('typed then closed')
		expect(second.handle.dirty).toBe(true)
	})

	it('keeps an edit typed while the row it re-reads behind is still going', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		let deployed = b
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => ({ deployed, draft: rows.sent.get('u/me/b') as Res | undefined }))
		)
		await settle()
		open.value = { ...b, description: 'typed first' }

		const flushing = deferred()
		rows.hold(flushing.promise)
		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => void (deployed = { ...ctx.value }))
		)
		await settle()
		panel.value = { ...b, args: { a: 2 } }
		const wrote = panel.save()
		await settle()
		// In neither the row going out nor the read coming back, and newer than both.
		const late = { ...b, description: 'typed while that row was going' }
		open.value = late
		flushing.resolve()

		expect(await wrote).toMatchObject({ ok: true, path: 'u/me/b' })
		expect(open.value).toEqual(late)
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
	})

	it('writes an item it moves onto after the saves queued there, and supersedes later ones', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const order: string[] = []
		const gate = deferred()
		const b = { ...deployedRes, path: 'u/me/b' }
		const { handle: other } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter({ deployed: b }, async (ctx) => {
				await gate.promise
				order.push(ctx.value.description)
			})
		)
		const temporary = newItemPath()
		const { handle: moving } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b },
			adapter({}, async (ctx) => void order.push(ctx.value.description))
		)
		await settle()

		other.value = { ...b, description: 'first' }
		const first = other.save()
		moving.value = { ...b, description: 'second' }
		const second = moving.save()
		await settle()
		other.value = { ...b, description: 'queued behind the move' }
		const third = other.save()
		await settle()
		expect(order).toEqual([])

		gate.resolve()
		expect(await first).toMatchObject({ ok: true })
		expect(await second).toMatchObject({ ok: true, moved: true })
		expect(await third).toMatchObject({ ok: false })
		expect(order).toEqual(['first', 'second'])
		expect(other.deployed?.description).toBe('second')
	})

	it('holds an editor that opens an item while a save is moving onto it', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const order: string[] = []
		const gate = deferred()
		const b = { ...deployedRes, path: 'u/me/b' }
		const temporary = newItemPath()
		const { handle: moving } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b },
			adapter({}, async (ctx) => {
				await gate.promise
				order.push(`moved: ${ctx.value.description}`)
			})
		)
		await settle()
		moving.value = { ...b, description: 'moving' }
		const moved = moving.save()
		await settle()

		const { handle: opened } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(
				async () => {
					order.push('loaded')
					return { deployed: b }
				},
				async (ctx) => void order.push(`saved: ${ctx.value.description}`)
			)
		)
		const savedMeanwhile = opened.save()
		await settle()
		expect(order).toEqual([])

		gate.resolve()
		expect(await moved).toMatchObject({ ok: true, moved: true })
		expect(await savedMeanwhile).toMatchObject({ ok: false })
		expect(order).toEqual(['moved: moving'])
		expect(opened.value?.description).toBe('moving')
	})

	it('holds an entry an earlier move put at a key while a later move is heading there', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const order: string[] = []
		const gate = deferred()
		const b = { ...deployedRes, path: 'u/me/b' }
		const creating = (write: (ctx: ItemWriteContext<Res>) => Promise<void>) => {
			const path = newItemPath()
			return store.acquire(
				{ workspace: 'w', kind: 'resource', path },
				{ workspace: 'w', path, template: b },
				adapter({}, write)
			).handle
		}
		const first = creating(async (ctx) => void order.push(ctx.value.description))
		const second = creating(async (ctx) => {
			await gate.promise
			order.push(ctx.value.description)
		})
		await settle()
		first.value = { ...b, description: 'first' }
		second.value = { ...b, description: 'second' }
		const firstMove = first.save()
		const secondMove = second.save()
		expect(await firstMove).toMatchObject({ ok: true, moved: true })

		first.value = { ...b, description: 'saved again through the first' }
		const later = first.save()
		await settle()
		expect(order).toEqual(['first'])

		gate.resolve()
		expect(await secondMove).toMatchObject({ ok: true, moved: true })
		expect(await later).toMatchObject({ ok: false })
		expect(order).toEqual(['first', 'second'])
		// Its handle follows the item to the entry that now holds the key.
		expect(first.deployed?.description).toBe('second')
		expect(first.value?.description).toBe('second')
	})
})

describe('item store: conflicts', () => {
	it('stops writing a key the server rejected, until the conflict is resolved', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }))
		item.value = { ...deployedRes, description: 'one' }
		expect(rows.writes).toHaveLength(1)

		rows.conflicts.add('u/me/r')
		expect(item.status).toBe('conflicted')
		item.value = { ...deployedRes, description: 'two' }
		item.value = structuredClone(deployedRes)
		expect(rows.writes).toHaveLength(1)

		item.value = { ...deployedRes, description: 'mine' }
		await item.resolveConflict('overwrite')
		expect(item.status).toBe('idle')
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'mine' }
		})
	})

	it('does not take the baseline of a first read the chat saved over', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		await settle()
		// The chat saves its own row for this key while the first read is still out. Nothing the
		// entry does can count that: it writes through the syncer, not through this entry.
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes, draft: deployedRes, draftSavedAt: 'T0' })
		await settle()

		// Taking T0 would put the syncer behind the chat's row and refuse every write after it.
		expect(rows.seeds).toEqual([])
		expect(item.status).not.toBe('conflicted')
	})

	it('leaves a conflicted editor alone when someone else writes its item', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		let deployed = b
		const theirs = { ...b, description: 'the other tab' }
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => ({ deployed, draft: theirs }))
		)
		await settle()
		open.value = { ...b, description: 'mine' }
		// This tab's row was rejected, so what is on screen is on screen only.
		rows.conflicts.add('u/me/b')
		expect(open.status).toBe('conflicted')

		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => void (deployed = { ...ctx.value }))
		)
		await settle()
		panel.value = { ...b, args: { a: 2 } }
		expect(await panel.save()).toMatchObject({ ok: true, path: 'u/me/b' })

		// The re-read would have replaced "mine" with the row the server holds and called the
		// conflict resolved. It is the user's to resolve, so it stays.
		expect(open.value?.description).toBe('mine')
		expect(open.status).toBe('conflicted')
		// The deployed side still learns the write: keeping mine and then discarding has to land
		// on what the server holds now, not on the config it held before the panel wrote.
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
		expect(await open.discard()).toEqual({ removed: false })
		expect(open.value).toEqual({ ...b, args: { a: 2 } })
	})

	it('keeps a local edit the flush before a re-read found a conflict for', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		let deployed = b
		const theirs = { ...b, description: 'the other tab' }
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => ({ deployed, draft: theirs }))
		)
		await settle()
		open.value = { ...b, description: 'mine' }
		// The row is queued, not yet refused: the conflict only surfaces when the flush sends it,
		// which is the flush the re-read does before reading.
		rows.conflictOnFlush.add('u/me/b')

		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => void (deployed = { ...ctx.value }))
		)
		await settle()
		panel.value = { ...b, args: { a: 2 } }
		expect(await panel.save()).toMatchObject({ ok: true, path: 'u/me/b' })

		expect(open.status).toBe('conflicted')
		expect(open.value?.description).toBe('mine')
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
	})

	it('keeps an edit the server never received when the item is reopened', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const a = adapter(async () => ({
			deployed: deployedRes,
			draft: rows.sent.get('u/me/r') as Res | undefined
		}))
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		// The autosave cannot reach the server at all: the row is parked for a later attempt.
		rows.failing.add('u/me/r')
		first.handle.value = { ...deployedRes, description: 'never reached the server' }
		await rows.port.flush(key)
		expect(rows.sent.has('u/me/r')).toBe(false)
		first.release()

		// Reopening reads afresh, and its GET works even though the draft POST does not.
		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// The parked payload is the only record of that edit. Dropping it loses it for good.
		expect(rows.dropped).not.toContain('u/me/r')
		rows.failing.delete('u/me/r')
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toEqual({
			...deployedRes,
			description: 'never reached the server'
		})
	})

	it('opens a conflicted item afresh, past the payload the server refused', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const theirs = { ...deployedRes, description: 'the other tab' }
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const a = adapter(async () => ({ deployed: deployedRes, draft: theirs, draftSavedAt: 'T2' }))
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		first.handle.value = { ...deployedRes, description: 'mine' }
		rows.conflictOnFlush.add('u/me/r')
		await rows.port.flush(key)
		expect(first.handle.status).toBe('conflicted')
		first.release()

		// Reopening reads afresh. Its own flush replays the refused payload, which must not make
		// the response that follows look stale — that would leave the editor conflicted forever.
		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		expect(second.handle.status).not.toBe('conflicted')
		expect(second.handle.value?.description).toBe('the other tab')

		// The baseline it just adopted is what would make the refused payload acceptable, so the
		// next flush — a page close, with nothing typed — must not resend it over their draft.
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toBeUndefined()

		second.handle.value = { ...deployedRes, description: 'typed after reopening' }
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed after reopening' }
		})
	})

	it('clears the conflict on a reload the user typed during', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const { item } = await open(
			rows,
			adapter(() => {
				reads++
				return reads > 1
					? read.promise
					: Promise.resolve({ deployed: deployedRes, draftSavedAt: 'T1' })
			})
		)
		item.value = { ...deployedRes, description: 'mine' }
		rows.conflicts.add('u/me/r')
		expect(item.status).toBe('conflicted')

		const resolving = item.resolveConflict('reload')
		await settle()
		// A conflicted key writes no row, so nothing typed here gets ahead of the read.
		item.value = { ...deployedRes, description: 'typed during the reload' }
		read.resolve({ deployed: deployedRes, draftSavedAt: 'T2' })
		await resolving

		expect(item.status).not.toBe('conflicted')
		expect(rows.seeds.at(-1)).toEqual({ path: 'u/me/r', at: 'T2' })
		expect(item.value?.description).toBe('typed during the reload')
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed during the reload' }
		})
	})

	it('drops the rejected payload when the other version is taken', async () => {
		const rows = fakeRows()
		const theirs = { ...deployedRes, description: 'theirs' }
		let reads = 0
		const { item } = await open(
			rows,
			adapter(async () =>
				reads++ === 0 ? { deployed: deployedRes } : { deployed: deployedRes, draft: theirs }
			)
		)
		item.value = { ...deployedRes, description: 'mine' }

		await item.resolveConflict('reload')
		expect(rows.dropped).toContain('u/me/r')
		expect(item.value).toEqual(theirs)
	})
})

describe('item store: settling', () => {
	it('folds the form settling into the deployed side until the first edit', async () => {
		const rows = fakeRows()
		const a: ItemAdapter<Res> = {
			...adapter({ deployed: deployedRes }),
			settles: true,
			absorbs: (next, deployed) => next.path === deployed.path
		}
		const { item } = await open(rows, a)
		item.value = { ...deployedRes, args: { a: 1, materialized: '' } }
		expect(item.dirty).toBe(false)
		expect(rows.writes).toEqual([])

		// A change the form cannot make on its own is the user's, gate or no gate.
		item.value = { ...deployedRes, path: 'u/me/moved', args: { a: 1, materialized: '' } }
		expect(item.dirty).toBe(true)
		expect(item.pristine).toBe(false)
	})
})
