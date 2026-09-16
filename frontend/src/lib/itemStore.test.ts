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
	/** Keys a reader is holding: nothing sends while one is out, and nothing is dropped. */
	const holds = new Map<string, number>()
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
	/** Paths the syncer holds a `last_sync` for. Without one a send is unconditional. */
	const baselines = new Set<string>()
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
			// queues behind it. With nothing queued, a payload parked for another attempt goes
			// instead.
			const fromQueue = queued.has(key.path)
			const going = fromQueue
				? { value: queued.get(key.path) }
				: parked.has(key.path)
					? { value: parked.get(key.path) }
					: undefined
			queued.delete(key.path)
			if (holdFlush) await holdFlush
			if (!going) return
			// Nothing goes out while the key is conflicted, whether it was queued before the
			// refusal or parked by it: the payload is kept so the edit survives, and which
			// version wins is the user's to say. `postSave` gates every send this way.
			if (conflicts.has(key.path) || holds.has(key.path)) {
				parked.set(key.path, going.value)
				return
			}
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
			// A save that lands gives the key a baseline from its own response, the way `postSave`
			// advances `lastSync` — or drops it, for a delete.
			if (going.value === null) baselines.delete(key.path)
			else baselines.add(key.path)
		},
		overwrite: async (key, value) => {
			conflicts.delete(key.path)
			writes.push({ path: key.path, value })
			handed(key.path)
		},
		rowMark: (key) => sends.get(key.path) ?? 0,
		// `pendingSaveOpts` holds the latest payload from the moment it is handed over, so a row
		// still in the debouncer counts as waiting just as much as one the server refused.
		pending: (key) =>
			queued.has(key.path)
				? queued.get(key.path)
				: parked.has(key.path)
					? parked.get(key.path)
					: undefined,
		unbased: (key) => (queued.has(key.path) || parked.has(key.path)) && !baselines.has(key.path),
		hold: (key) => {
			holds.set(key.path, (holds.get(key.path) ?? 0) + 1)
			let released = false
			return () => {
				if (released) return
				released = true
				const left = (holds.get(key.path) ?? 1) - 1
				if (left > 0) holds.set(key.path, left)
				else holds.delete(key.path)
			}
		},
		markConflict: (key) => void conflicts.add(key.path),
		seedSync: (key, at, since) => {
			// The syncer refuses a baseline from a read a row handed over since has passed.
			if (since !== (sends.get(key.path) ?? 0)) return
			seeds.push({ path: key.path, at })
			if (at !== undefined) baselines.add(key.path)
			else baselines.delete(key.path)
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
			// Cancels the debounce as well as forgetting the refused payload: both live in
			// `pendingSaveOpts`, and leaving the queued one would resurrect what was dropped.
			parked.delete(key.path)
			queued.delete(key.path)
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
	it('takes a new deployed value after an edit was put back, without drafting the old one', async () => {
		const rows = fakeRows()
		let deployed = deployedRes
		const { item } = await open(
			rows,
			adapter(async () => ({ deployed }))
		)
		// Edited and then put back by hand. Nothing here is the user's any more, so this editor
		// speaks for the item no differently than one just opened.
		item.value = { ...deployedRes, description: 'edited' }
		item.value = structuredClone(deployedRes)
		expect(item.dirty).toBe(false)

		deployed = { ...deployedRes, description: 'deployed elsewhere' }
		const writesBefore = rows.writes.length
		await item.reload()

		// Holding the old value against the new baseline would read as an unsaved change nobody
		// made, and post it as a draft over whatever is there.
		expect(item.value).toEqual(deployed)
		expect(item.dirty).toBe(false)
		expect(rows.writes.length).toBe(writesBefore)
	})

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
		// `persistGlobalDraft` hands the value here, says it is writing the row for it, and does.
		store.bridge.noteRow('w', 'resource', 'u/me/r')
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })

		expect(await discarding).toEqual({ removed: false })
		expect(item.value).toEqual(fromChat)
		expect(item.dirty).toBe(true)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: fromChat })
	})

	it('settles its own row when its draft is deployed, keeping anything typed during it', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const drafted = { ...deployedRes, description: 'the draft being deployed' }
		let deployed = deployedRes
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(async () => ({ deployed, draft: (rows.sent.get('u/me/r') as Res) ?? drafted }))
		)
		await settle()
		expect(item.value).toEqual(drafted)

		// The deploy writes the draft to the item, then asks the editor to re-read.
		deployed = drafted
		expect(await item.reload()).toEqual({ ok: true })
		// Nothing was typed during it, so the row it was holding is now redundant and goes on its
		// own. This is the cleanup, and is why the deploy must not discard on top of it.
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })

		// Now the same again with an edit made while the deploy was in flight.
		const typed = { ...deployedRes, description: 'typed while deploying' }
		item.value = typed
		await rows.port.flush(key)
		deployed = typed
		expect(await item.reload()).toEqual({ ok: true })
		expect(item.value).toEqual(typed)
	})

	it('keeps an edit typed after the chat asked for a discard, while a save held it up', async () => {
		const rows = fakeRows()
		const gate = deferred()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter({ deployed: deployedRes }, () => gate.promise)
		)
		await settle()
		item.value = { ...deployedRes, description: 'edited' }
		const saving = item.save()
		await settle()

		// The chat asks for the discard while the save is still out, then the user types.
		const discarding = store.bridge.discard('w', 'resource', 'u/me/r')
		await settle()
		item.value = { ...deployedRes, description: 'typed after the chat asked' }
		gate.resolve()
		await saving

		// The discard was about the value the chat saw, not this one. It reports the row dealt
		// with either way, so the chat does not delete what it is now holding.
		expect(await discarding).toBe('done')
		expect(item.value?.description).toBe('typed after the chat asked')
		expect(item.dirty).toBe(true)
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed after the chat asked' }
		})
	})

	it('keeps a value the chat persisted while a read that predates it was in flight', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const store = createItemStore(rows.port)
		const onScreen = { ...deployedRes, description: 'what the editor is showing' }
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => {
				if (reads++ === 0) return Promise.resolve({ deployed: deployedRes, draft: onScreen })
				return read.promise
			})
		)
		await settle()
		expect(item.value).toEqual(onScreen)

		const reloading = item.reload()
		await settle()
		// The chat persists exactly what is on screen. Nothing the user can see changes, but the
		// value is now a row on the server that the read in flight knows nothing about.
		item.applyExternal({ ...onScreen })
		rows.handExternally('u/me/r')
		// The read predates that, and answers with the newly deployed value and no draft.
		const deployedSince = { ...deployedRes, description: 'deployed while the read was out' }
		read.resolve({ deployed: deployedSince })
		await reloading

		// Answering with the deployed value here would hide a draft that exists, and the next
		// edit would go out over it on a baseline this read had just made fresh.
		expect(item.value).toEqual(onScreen)
		expect(item.deployed).toEqual(deployedSince)
		expect(item.dirty).toBe(true)
	})

	it('is left unable to act when its post-deploy re-read fails', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		let reads = 0
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(async () => {
				if (reads++ > 0) throw new Error('the GET failed')
				return { deployed: deployedRes }
			})
		)
		await settle()
		item.value = { ...deployedRes, description: 'typed during the deploy' }

		expect(await store.bridge.refresh('w', 'resource', 'u/me/r')).toBe('failed')
		// The deploy landed and this never saw it, so acting on the old baseline would undo it.
		expect(item.canSave).toBe(false)
		expect(await item.save()).toMatchObject({ ok: false })
		expect(await item.discard()).toEqual({ removed: false })
		expect(item.value?.description).toBe('typed during the deploy')
	})

	it('still autosaves after the chat wrote a row during its first read', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		await settle()
		// The chat writes while the GET is out. Its row is this entry's own doing, and its value
		// is right here — so the read is behind the server, but not in a way that loses anything.
		const fromChat = { ...deployedRes, description: 'from the chat' }
		item.applyExternal(fromChat)
		// `persistGlobalDraft` hands the value here and writes its own row through the syncer.
		store.bridge.noteRow('w', 'resource', 'u/me/r')
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })
		await settle()

		expect(item.value).toEqual(fromChat)
		// The editor opened on the chat's draft; typing from here has to keep reaching the row.
		expect(item.canSave).toBe(true)
		const typed = { ...deployedRes, description: 'typed after opening' }
		item.value = typed
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: typed })
	})

	it('still autosaves after the chat changed a loaded item during a reload', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => (reads++ === 0 ? Promise.resolve({ deployed: deployedRes }) : read.promise))
		)
		await settle()

		const reloading = item.reload()
		await settle()
		// `persistGlobalDraft` on a loaded item: the value goes in through the bridge, whose
		// reconcile writes a row, and then it writes its own row for that same value.
		const fromChat = { ...deployedRes, description: 'from the chat' }
		item.applyExternal(fromChat)
		store.bridge.noteRow('w', 'resource', 'u/me/r')
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })
		await reloading

		// Two rows, but both carry the value that is right here, so nothing is unaccounted for
		// and the editor has to stay able to persist what is typed next.
		expect(item.value).toEqual(fromChat)
		expect(item.canSave).toBe(true)
		const typed = { ...deployedRes, description: 'typed after the chat wrote' }
		item.value = typed
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: typed })
	})

	it('reports a re-read that answered but left the item behind as not refreshed', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => (reads++ === 0 ? Promise.resolve({ deployed: deployedRes }) : read.promise))
		)
		await settle()

		const refreshing = store.bridge.refresh('w', 'resource', 'u/me/r')
		await settle()
		// The GET works, but another surface's row lands while it is out.
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })

		// It answered, so `ok` is true — but the item is behind a row it never saw, and a caller
		// told "done" would go on to report a draft removed that is still there.
		expect(await refreshing).toBe('failed')
		expect(item.canSave).toBe(false)
	})

	it("does not credit an edit made during a re-read's own flush", async () => {
		const rows = fakeRows()
		const flushing = deferred()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => (reads++ === 0 ? Promise.resolve({ deployed: deployedRes }) : read.promise))
		)
		await settle()
		item.value = { ...deployedRes, description: 'edited' }
		rows.hold(flushing.promise)

		const refreshing = store.bridge.refresh('w', 'resource', 'u/me/r')
		await settle()
		// Typed while the re-read's prerequisite flush is still going: its row goes out here,
		// before the read starts, so it is not one of the rows the read has to account for.
		item.value = { ...deployedRes, description: 'typed during the flush' }
		flushing.resolve()
		await settle()
		// Then a row nobody here has, while the GET is out.
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })
		await refreshing

		// The earlier edit must not pay for that row: this editor is behind and has to reload.
		expect(item.canSave).toBe(false)
	})

	it('is not fooled by the chat crediting a row a loading item never wrote', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		await settle()

		// The chat seeds and persists while the first GET is out. Loading, this entry writes no
		// row of its own, so the chat's save is the only row that value produced.
		item.applyExternal({ ...deployedRes, description: 'from the chat' })
		store.bridge.noteRow('w', 'resource', 'u/me/r')
		rows.handExternally('u/me/r')
		// The legacy agent editor then writes the same resource: a row nobody here has.
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })
		await settle()

		// Two rows, one of them unaccounted for. Crediting the chat twice would hide it and let
		// this editor post over the agent editor's newer draft.
		expect(item.canSave).toBe(false)
	})

	it('does not credit a row to a seed that wrote none', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		await settle()

		// The agent editor seeds its restored draft here while the GET is out. Loading, this
		// writes no row, and the seeding editor writes none for it either.
		const seeded = { ...deployedRes, description: 'restored by the agent editor' }
		item.applyExternal(seeded)
		// It then autosaves something newer through the `UserDraft` handle it kept: a row whose
		// value never came through here.
		rows.handExternally('u/me/r')
		const writesBefore = rows.writes.length
		read.resolve({ deployed: deployedRes })
		await settle()

		// One row, nothing accounting for it. Posting the seed now would put it on the baseline
		// that row advanced, silently replacing it.
		expect(item.canSave).toBe(false)
		expect(rows.writes.length).toBe(writesBefore)
	})

	it('stays blocked when an unseen row follows the edit it kept', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		let reads = 0
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => (reads++ === 0 ? Promise.resolve({ deployed: deployedRes }) : read.promise))
		)
		await settle()

		const reloading = item.reload()
		await settle()
		// The user types while the GET is out: newer than the read, and its own row goes out.
		item.value = { ...deployedRes, description: 'mine' }
		// Then another draft surface writes this key directly, later than that edit.
		rows.handExternally('u/me/r')
		read.resolve({ deployed: deployedRes })
		await reloading

		// A value did arrive during the read, but the unseen row came after it, so what is held
		// here is older than what the server has. Saving it would deploy the older of the two.
		expect(item.value?.description).toBe('mine')
		expect(item.canSave).toBe(false)
	})

	it('does not post a read back over a row another editor landed during it', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const older = { ...deployedRes, description: 'draft A, as the GET found it' }
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		await settle()
		// Another editor of the same item saves draft B through the syncer directly and it lands,
		// leaving nothing pending. Only the row mark records that it happened.
		rows.handExternally('u/me/r')
		const writesBefore = rows.writes.length
		read.resolve({ deployed: deployedRes, draft: older, draftSavedAt: 'T0' })
		await settle()

		// Posting A now would overwrite B, on the baseline B itself advanced.
		expect(rows.writes.slice(writesBefore)).toEqual([])
		expect(rows.seeds).toEqual([])
		// And nothing may be written from this view until it is reloaded.
		expect(item.canSave).toBe(false)
		item.value = { ...deployedRes, description: 'typed on a stale view' }
		expect(rows.writes.slice(writesBefore)).toEqual([])
	})

	it('tells a caller its post-deploy re-read failed, apart from never having the item', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		let reads = 0
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(async () => {
				// The first read works, so this editor has the item; the one after the deploy does not.
				if (reads++ > 0) throw new Error('the GET failed')
				return { deployed: deployedRes }
			})
		)
		await settle()
		item.value = { ...deployedRes, description: 'typed during the deploy' }

		// `absent` would send the caller off to delete this row and discard against a baseline
		// from before the deploy; the editor still has the item, so it says so.
		expect(await store.bridge.refresh('w', 'resource', 'u/me/r')).toBe('failed')
		expect(item.value?.description).toBe('typed during the deploy')
		expect(item.dirty).toBe(true)
	})

	it('gives a refused toggle its field back while an edit made meanwhile stays', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const refusal = deferred()
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ deployed: { path: 's', enabled: true, summary: 'a' } })
			} as ItemAdapter<Sched>
		)
		await settle()
		const toggling = item.patch({ enabled: false }, () => refusal.promise)
		// The user types in another field while the request is out.
		item.value = { path: 's', enabled: false, summary: 'typed while disabling' }
		refusal.reject(new Error('refused'))

		expect(await toggling).toMatchObject({ ok: false })
		// The server stayed enabled, so showing it disabled — and drafting that — would be an
		// editor claiming a schedule is off while it is still running. The summary is untouched.
		expect(item.value).toEqual({ path: 's', enabled: true, summary: 'typed while disabling' })
		expect(rows.writes.at(-1)).toEqual({
			path: 's',
			value: { path: 's', enabled: true, summary: 'typed while disabling' }
		})
	})

	it('does not roll a refused toggle back over a draft written while it was out', async () => {
		type Sched = { path: string; enabled: boolean; summary: string }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const refusal = deferred()
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'trigger_schedule', path: 's' },
			{ workspace: 'w', path: 's' },
			{
				load: async () => ({ deployed: { path: 's', enabled: false, summary: 'a' } })
			} as ItemAdapter<Sched>
		)
		await settle()
		const toggling = item.patch({ enabled: true }, () => refusal.promise)
		// The chat writes a draft that happens to carry the same `enabled` the toggle sent.
		item.applyExternal({ path: 's', enabled: true, summary: 'from the chat' })
		refusal.reject(new Error('refused'))

		expect(await toggling).toMatchObject({ ok: false })
		// Equality with what the toggle sent is not proof the toggle put it there.
		expect(item.value).toEqual({ path: 's', enabled: true, summary: 'from the chat' })
		expect(rows.writes.at(-1)).toEqual({
			path: 's',
			value: { path: 's', enabled: true, summary: 'from the chat' }
		})
	})

	it('owns the row a load queued in front of its discard turned up', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => read.promise)
		)
		// The chat discards while that first GET is still out, so the discard queues behind it.
		const discarding = store.bridge.discard('w', 'resource', 'u/me/r')
		await settle()
		read.resolve({ deployed: deployedRes, draft: { ...deployedRes, description: 'draft A' } })

		// The load turns up a draft and the discard deletes it, so this editor did deal with the
		// row. Answering `absent` would send a second, unconditional delete after that one.
		expect(await discarding).toBe('done')
		expect(item.dirty).toBe(false)
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
	})

	it('says a row it never knew of is not its to discard', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		// Loaded clean: no draft existed when this editor read the item.
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter({ deployed: deployedRes })
		)
		await settle()
		expect(item.dirty).toBe(false)

		// Another tab creates the draft afterwards. This editor knows nothing of it.
		rows.sent.set('u/me/r', { ...deployedRes, description: 'the other tab' })
		rows.handExternally('u/me/r')

		// Its discard does nothing, so answering `done` would have the caller skip the delete and
		// report a draft gone that is still on the server.
		expect(await store.bridge.discard('w', 'resource', 'u/me/r')).toBe('absent')
	})

	it('says it did not deal with the row when its own first read never arrived', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter(async () => {
				throw new Error('the GET failed')
			})
		)
		await settle()
		expect(item.loaded).toBe(false)

		// It is registered, so the chat would otherwise take its answer as the cleanup being done
		// — while an entry with nothing loaded writes no delete at all and the row survives.
		expect(await store.bridge.discard('w', 'resource', 'u/me/r')).toBe('absent')
		expect(await store.bridge.refresh('w', 'resource', 'u/me/r')).toBe('absent')
		expect(rows.writes).toEqual([])
	})

	it('reports an item deleted elsewhere as gone, not as clean', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/r' },
			{ workspace: 'w', path: 'u/me/r' },
			adapter({ deployed: deployedRes })
		)
		await settle()
		item.value = { ...deployedRes, description: 'edited' }
		expect(item.dirty).toBe(true)

		await store.bridge.itemDeleted('w', 'resource', 'u/me/r')

		// Resetting to the deployed baseline would leave the editor clean on something that no
		// longer exists, and its next save would 404.
		expect(item.removed).toBe(true)
		expect(item.value).toBeUndefined()
		expect(item.deployed).toBeUndefined()
		expect(rows.hints.get('u/me/r')).toBe(false)
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

	it('does not let a stale discard delete the draft its recovery turned up', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const recovery = deferred<ItemLoad<Res>>()
		let reads = 0
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const mine = { ...deployedRes, description: 'draft A, mine' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => {
				reads++
				if (reads === 1) return Promise.resolve({ deployed: deployedRes, draft: mine })
				if (reads === 2) return Promise.reject(new Error('the GET failed'))
				return recovery.promise
			})
		)
		await settle()
		expect(item.value).toEqual(mine)
		// A write landed elsewhere and the re-read for it failed, so this editor is stale.
		expect(await store.bridge.refresh('w', 'resource', 'u/me/r')).toBe('failed')

		// The chat discards A. Stale refuses, so it reloads to recover — and that read finds
		// draft B, which another tab wrote and nobody asked to discard.
		const discarding = store.bridge.discard('w', 'resource', 'u/me/r')
		await settle()
		const theirs = { ...deployedRes, description: 'draft B, theirs' }
		const writesBefore = rows.writes.length
		recovery.resolve({ deployed: deployedRes, draft: theirs, draftSavedAt: 'T-b' })

		// Deleting B here would take a newer draft past the conflict A's own baseline would have
		// raised. The discard reports it could not be done instead.
		expect(await discarding).toBe('failed')
		expect(item.value).toEqual(theirs)
		expect(rows.writes.slice(writesBefore)).toEqual([])
	})

	it('keeps an edit typed while a stale discard was recovering the item', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const recovery = deferred<ItemLoad<Res>>()
		let reads = 0
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const { handle: item } = store.acquire(
			key,
			{ workspace: 'w', path: 'u/me/r' },
			adapter(() => {
				reads++
				if (reads === 1) return Promise.resolve({ deployed: deployedRes })
				if (reads === 2) return Promise.reject(new Error('the GET failed'))
				return recovery.promise
			})
		)
		await settle()
		item.value = { ...deployedRes, description: 'mine' }
		// A write landed elsewhere and the re-read for it failed, so this editor is stale.
		expect(await store.bridge.refresh('w', 'resource', 'u/me/r')).toBe('failed')

		// The chat discards. That is refused while stale, so it reloads to recover — and the
		// user types while that GET is out.
		const discarding = store.bridge.discard('w', 'resource', 'u/me/r')
		await settle()
		item.value = { ...deployedRes, description: 'typed during the recovery' }
		recovery.resolve({ deployed: deployedRes })

		// The retry is about the value the chat named, not this one, so the edit stands.
		expect(await discarding).toBe('done')
		expect(item.value?.description).toBe('typed during the recovery')
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed during the recovery' }
		})
	})

	it('stops an editor acting on a baseline its failed re-read left behind', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const b = { ...deployedRes, path: 'u/me/b' }
		let deployed = b
		let reads = 0
		const { handle: open } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: 'u/me/b' },
			{ workspace: 'w', path: 'u/me/b' },
			adapter(async () => {
				// The first read works; the one the panel's write triggers does not.
				if (reads++ > 0) throw new Error('the GET failed')
				return { deployed }
			})
		)
		await settle()
		open.value = { ...b, description: 'mine' }

		const temporary = newItemPath()
		const { handle: panel } = store.acquire(
			{ workspace: 'w', kind: 'resource', path: temporary },
			{ workspace: 'w', path: temporary, template: b, standsFor: 'u/me/b' },
			adapter({}, async (ctx) => void (deployed = { ...ctx.value }))
		)
		await settle()
		panel.value = { ...b, args: { a: 2 } }
		// The write lands: the panel's success does not depend on the other editor re-reading.
		expect(await panel.save()).toMatchObject({ ok: true, path: 'u/me/b' })

		// But that editor is now a baseline behind, so saving or discarding would put the old
		// config back over the write. Neither is offered until it reloads.
		expect(open.canSave).toBe(false)
		expect(await open.discard()).toEqual({ removed: false })
		expect(open.value?.description).toBe('mine')

		// The gate is in `save` itself, not only in `canSave`: not every editor consults that.
		expect(await open.save()).toMatchObject({ ok: false })
		expect(deployed).toEqual({ ...b, args: { a: 2 } })
		// An outside discard reloads to clear the staleness; that GET fails too, so it reports
		// the draft still there rather than letting the caller delete the only copy of it.
		expect(await store.bridge.discard('w', 'resource', 'u/me/b')).toBe('failed')
		expect(open.value?.description).toBe('mine')

		// A reload gets it the baseline it missed, and it can act again.
		reads = 0
		expect(await open.reload()).toEqual({ ok: true })
		expect(open.deployed).toEqual({ ...b, args: { a: 2 } })
		open.value = { ...b, args: { a: 2 }, description: 'edited after reloading' }
		expect(open.canSave).toBe(true)
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

	it('does not fold a secret it fetched over a value written since', async () => {
		type Var = { path: string; value: string; is_secret: boolean }
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const decrypting = deferred<(side: Var) => void>()
		const { handle: item } = store.acquire(
			{ workspace: 'w', kind: 'variable', path: 'u/me/v' },
			{ workspace: 'w', path: 'u/me/v' },
			{
				load: async () => ({
					deployed: { path: 'u/me/v', value: '$encrypted:xx', is_secret: true }
				})
			} as ItemAdapter<Var>
		)
		await settle()

		const learning = item.learn(() => decrypting.promise)
		await settle()
		// The chat writes a newer secret while the decryption request is out.
		item.applyExternal({ path: 'u/me/v', value: 'the new secret', is_secret: true })
		decrypting.resolve((side) => void (side.value = 'the old secret'))

		expect(await learning).toEqual({ ok: true })
		// What it decrypted is the deployed secret from before that write. Folding it into the
		// value would put the old one back on screen and autosave it over the new one.
		expect(item.value?.value).toBe('the new secret')
		expect(item.deployed?.value).toBe('the old secret')
	})

	it('keeps a row that landed while its reopening read was in flight', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const mine = { ...deployedRes, description: 'queued, not yet sent' }
		const a = adapter(() => read.promise)
		// A first autosave is queued and unsent, so the key has no baseline yet.
		rows.port.write(key, mine)
		const acq = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		// Its debounce fires and lands while the reopening GET is still out.
		await rows.port.flush(key)
		// The GET predates that, so it reports no draft at all.
		read.resolve({ deployed: deployedRes })
		await settle()

		// Answering with the deployed value would hide the draft that just landed and reseed the
		// baseline its own save had set. What was waiting when the read went out is what stands.
		expect(acq.handle.value).toEqual(mine)
		expect(rows.seeds).toEqual([])
	})

	it('holds its own unbased autosave across the reopening read rather than racing it', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const mine = { ...deployedRes, description: 'typed before closing' }
		const a = adapter(() => read.promise)
		// No draft exists yet, so this first autosave carries no baseline: sending it is
		// unconditional, and nothing yet knows whether there is a row for it to take over.
		rows.port.write(key, mine)
		const acq = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		// Its debounce fires while the reopening GET is still out.
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toBeUndefined()

		// The read says there is no row, so there was never anything to take over — and nothing to
		// conflict against either, which would have stopped the editor saving with nobody else
		// involved.
		read.resolve({ deployed: deployedRes })
		await settle()
		expect(acq.handle.status).not.toBe('conflicted')
		expect(acq.handle.value).toEqual(mine)

		// Released with the read, the edit goes out as it always would have.
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toEqual(mine)
	})

	it('does not let a pending send fire while the reopening read is still out', async () => {
		const rows = fakeRows()
		const read = deferred<ItemLoad<Res>>()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const mine = { ...deployedRes, description: 'tab A, never sent' }
		const a = adapter(() => read.promise)
		// Unbased again: written when no row existed, so it would go out unconditional.
		rows.port.write(key, mine)
		const acq = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// Another tab creates the draft while this read is still unresolved, and only then does
		// this tab's debounce fire.
		const theirs = { ...deployedRes, description: 'tab B' }
		rows.sent.set('u/me/r', theirs)
		rows.handExternally('u/me/r')
		await rows.port.flush(key)

		// Sending here would have gone out with no `last_sync`, which the server takes
		// unconditionally — over their draft, and before this read could say it was there.
		expect(rows.sent.get('u/me/r')).toEqual(theirs)

		read.resolve({ deployed: deployedRes, draft: theirs, draftSavedAt: 'T-theirs' })
		await settle()
		expect(acq.handle.status).toBe('conflicted')
		expect(rows.sent.get('u/me/r')).toEqual(theirs)
	})

	it('does not let a debounce armed before a synthetic conflict send behind the user', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const a = adapter(async () => {
			const draft = rows.sent.get('u/me/r') as Res | undefined
			return { deployed: deployedRes, draft, draftSavedAt: draft ? 'T-theirs' : undefined }
		})
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		// No draft exists, so this edit is queued with no baseline behind it. It is never
		// attempted: the editor closes while it is still sitting in the debounce.
		first.handle.value = { ...deployedRes, description: 'tab A, still debouncing' }
		first.release()

		// Another tab creates the draft meanwhile.
		const theirs = { ...deployedRes, description: 'tab B' }
		rows.sent.set('u/me/r', theirs)
		rows.handExternally('u/me/r')

		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		// Two drafts and no baseline to order them by, so the read raises the conflict itself.
		expect(second.handle.status).toBe('conflicted')

		// The debounce armed before any of that is still holding tab A's edit. Firing it would
		// go out unconditional, take their draft with it, and answer the conflict nobody settled.
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toEqual(theirs)
		expect(second.handle.status).toBe('conflicted')
	})

	it('does not replay a first failed autosave over a draft created since', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		// No draft exists, so nothing gives this key a baseline. Once one does, the read carries
		// its timestamp the way the endpoint does.
		const a = adapter(async () => {
			const draft = rows.sent.get('u/me/r') as Res | undefined
			return { deployed: deployedRes, draft, draftSavedAt: draft ? 'T-theirs' : undefined }
		})
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		rows.failing.add('u/me/r')
		first.handle.value = { ...deployedRes, description: 'tab A, never sent' }
		await rows.port.flush(key)
		first.release()

		// Another tab creates the draft while tab A is away.
		const theirs = { ...deployedRes, description: 'tab B' }
		rows.sent.set('u/me/r', theirs)
		rows.handExternally('u/me/r')
		// Tab A's network recovers before it reopens.
		rows.failing.delete('u/me/r')

		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// The parked payload has no `last_sync`, so replaying it before the read would go out
		// unconditional and take their draft with it — and the read after would see only the
		// overwrite, never a conflict.
		expect(rows.sent.get('u/me/r')).toEqual(theirs)
		// Tab A's edit is not lost either: it is what the reopened editor shows.
		expect(second.handle.value?.description).toBe('tab A, never sent')
		// Two divergent drafts and no baseline to order them by, so it is the user's to settle —
		// and until they do, nothing goes out. A page close must not decide it for them.
		expect(second.handle.status).toBe('conflicted')
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toEqual(theirs)
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

		// The parked payload is the only record of that edit, so the reopened editor has to be
		// showing it: anything else and the next keystroke replaces it with nobody the wiser.
		expect(rows.dropped).not.toContain('u/me/r')
		expect(second.handle.value?.description).toBe('never reached the server')
		expect(second.handle.dirty).toBe(true)

		// Typing now builds on what is on screen, and that is what eventually lands.
		second.handle.value = { ...deployedRes, description: 'and then some more' }
		rows.failing.delete('u/me/r')
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toEqual({
			...deployedRes,
			description: 'and then some more'
		})
	})

	it('does not rebase an unsent edit onto a draft written since', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		// Another tab saved B while our A sat parked, so the server's row is theirs now.
		const theirs = { ...deployedRes, description: 'saved by another tab' }
		const a = adapter(async () => ({ deployed: deployedRes, draft: theirs, draftSavedAt: 'T9' }))
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		rows.failing.add('u/me/r')
		first.handle.value = { ...deployedRes, description: 'mine, never sent' }
		await rows.port.flush(key)
		first.release()

		const seedsBefore = rows.seeds.length
		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// Ours is shown, because it is the only copy of it. But taking their timestamp would let
		// it be accepted over their draft with no conflict ever raised, so it is not taken.
		expect(second.handle.value?.description).toBe('mine, never sent')
		expect(rows.seeds.slice(seedsBefore)).toEqual([])
	})

	it('re-issues a draft delete the server never received, on reopening', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const draft = { ...deployedRes, description: 'a draft to be discarded' }
		// The server keeps the draft throughout: the delete never reaches it.
		const a = adapter(async () => ({ deployed: deployedRes, draft }))
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		rows.failing.add('u/me/r')
		expect(await first.handle.discard()).toEqual({ removed: false })
		expect(rows.sent.has('u/me/r')).toBe(false)
		first.release()
		const writesBefore = rows.writes.length

		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// Reopening must not read the parked delete as "there is no draft": the row is still the
		// server's, so the delete has to go out again rather than be quietly forgotten.
		expect(second.handle.value).toEqual(deployedRes)
		expect(rows.writes.slice(writesBefore)).toEqual([{ path: 'u/me/r', value: null }])
		rows.failing.delete('u/me/r')
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toBeNull()
	})

	it('keeps the failed edit its own reopening retry got refused', async () => {
		const rows = fakeRows()
		const store = createItemStore(rows.port)
		const key: ItemKey = { workspace: 'w', kind: 'resource', path: 'u/me/r' }
		const a = adapter(async () => ({
			deployed: deployedRes,
			draft: rows.sent.get('u/me/r') as Res | undefined,
			draftSavedAt: 'T1'
		}))
		// A draft exists, so this key has a baseline and its autosave is not unconditional.
		rows.sent.set('u/me/r', { ...deployedRes, description: 'the first draft' })
		const first = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		rows.failing.add('u/me/r')
		const mine = { ...deployedRes, description: 'mine, never sent' }
		first.handle.value = mine
		await rows.port.flush(key)
		first.release()

		// Another tab writes a newer draft, and this tab's network recovers. Reopening retries
		// the parked edit before reading, and that retry is what the server refuses.
		rows.sent.set('u/me/r', { ...deployedRes, description: 'the other tab' })
		rows.failing.delete('u/me/r')
		rows.conflictOnFlush.add('u/me/r')

		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		// That edit is the only copy anyone has. Dropping it for the remote draft and clearing
		// the conflict on the way past would lose it with nothing said.
		expect(second.handle.value).toEqual(mine)
		expect(second.handle.status).toBe('conflicted')

		// Closing the drawer without answering the conflict must not answer it either. The edit
		// lives in the parked payload rather than in the entry, so it outlives the entry, and
		// the next opening still has both versions to offer.
		second.release()
		const third = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()
		expect(third.handle.value).toEqual(mine)
		expect(third.handle.status).toBe('conflicted')
		// And nothing went out behind the user in the meantime.
		expect(rows.sent.get('u/me/r')).toEqual({ ...deployedRes, description: 'the other tab' })
	})

	it('opens a conflicted item on the refused payload, and moves on once it is resolved', async () => {
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

		// Reopening is not an answer to the conflict. The refused payload is still the only copy
		// of that edit, so it is what the editor opens on, and nothing goes out — a page close
		// with nothing typed must not settle it either way.
		const second = store.acquire(key, { workspace: 'w', path: 'u/me/r' }, a)
		await settle()

		expect(second.handle.status).toBe('conflicted')
		expect(second.handle.value?.description).toBe('mine')
		await rows.port.flush(key)
		expect(rows.sent.get('u/me/r')).toBeUndefined()

		// Taking their version is the answer, and the editor writes normally again afterwards.
		await second.handle.resolveConflict('reload')
		expect(second.handle.status).not.toBe('conflicted')
		expect(second.handle.value?.description).toBe('the other tab')

		second.handle.value = { ...deployedRes, description: 'typed after resolving' }
		expect(rows.writes.at(-1)).toEqual({
			path: 'u/me/r',
			value: { ...deployedRes, description: 'typed after resolving' }
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
