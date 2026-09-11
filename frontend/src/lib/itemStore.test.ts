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
	const hints = new Map<string, boolean>()
	const port: ItemRowPort = {
		write: (key, value) => void writes.push({ path: key.path, value }),
		flush: async () => {},
		overwrite: async (key, value) => {
			conflicts.delete(key.path)
			writes.push({ path: key.path, value })
		},
		seedSync: () => {},
		conflicted: (key) => conflicts.has(key.path),
		failure: () => undefined,
		dropPending: () => {},
		hint: (key, on) => void hints.set(key.path, on)
	}
	return { port, writes, conflicts, hints }
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
		const outcome = await item.patch({ enabled: false }, async () => {
			throw new Error('refused')
		})
		expect(outcome).toEqual({ ok: false, error: 'refused' })
		expect(item.value).toEqual({ path: 's', enabled: true })
		expect(item.dirty).toBe(false)
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

	it('creates under a temporary path, then moves to the real one with no row at either', async () => {
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
		expect(rows.writes).toEqual([])
		expect(store.bridge.read('w', 'resource', 'u/me/created')).toEqual({ value: undefined })
		expect(store.bridge.read('w', 'resource', temp)).toBeUndefined()
	})

	it('moves a renamed item and deletes the row it left behind', async () => {
		const rows = fakeRows()
		const { item } = await open(rows, adapter({ deployed: deployedRes }))
		item.value = { ...deployedRes, path: 'u/me/renamed' }
		expect(rows.writes).toEqual([
			{ path: 'u/me/r', value: { ...deployedRes, path: 'u/me/renamed' } }
		])

		expect(await item.save()).toEqual({ ok: true, path: 'u/me/renamed', moved: true })
		expect(rows.writes.at(-1)).toEqual({ path: 'u/me/r', value: null })
		expect(rows.writes.some((w) => w.path === 'u/me/renamed')).toBe(false)
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
