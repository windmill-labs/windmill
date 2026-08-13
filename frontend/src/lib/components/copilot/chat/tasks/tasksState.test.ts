import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { SessionTasksStore, summarizeTasks } from './tasksState.svelte'

// The user-scoping subscription is BROWSER-gated; the node test env reports false.
vi.mock('esm-env', async (orig) => ({
	...(await orig<typeof import('esm-env')>()),
	BROWSER: true
}))

// Stub $lib/stores + $lib/utils (userScopedStorage's only deps here) to keep their heavy
// svelte/app-store graphs out of the per-test cold transform.
vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { userStore: writable(undefined) }
})
vi.mock('$lib/utils', () => ({ getLocalSetting: () => undefined, storeLocalSetting: () => {} }))

// The DB module memoises its handle at module scope. A fresh IDBFactory per test only
// isolates data once the handle is reset, so reset modules and re-import both together.
// The DB is namespaced by email, so seed a user.
async function fresh() {
	vi.resetModules()
	;(globalThis as any).indexedDB = new IDBFactory()
	;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
	const { SessionTasksStore: Store } = await import('./tasksState.svelte')
	const db = await import('./tasksDB')
	return { store: new Store(), makeStore: () => new Store(), db }
}

let store: SessionTasksStore
let makeStore: () => SessionTasksStore
let db: typeof import('./tasksDB')
beforeEach(async () => {
	;({ store, makeStore, db } = await fresh())
})

const mk = (subject: string) => ({ subject, description: `do ${subject}` })

describe('SessionTasksStore', () => {
	it('numbers tasks per session, continuing past the highest existing seq', async () => {
		await store.setSession('s1')
		expect((await store.createMany('s1', [mk('a'), mk('b')])).map((t) => t.seq)).toEqual([1, 2])
		expect((await store.createMany('s1', [mk('c')])).map((t) => t.seq)).toEqual([3])

		// A second session numbers from 1 again — seq is only unique within a session,
		// which is what makes the compound [sessionId, seq] key necessary.
		await store.setSession('s2')
		expect((await store.createMany('s2', [mk('x')])).map((t) => t.seq)).toEqual([1])
		expect(store.tasks.map((t) => t.subject)).toEqual(['x'])
	})

	it('keeps seq order regardless of update order, and scopes updates by session', async () => {
		await store.setSession('s1')
		await store.createMany('s1', [mk('a'), mk('b'), mk('c')])
		await store.createMany('s2', [mk('other')])

		await store.update('s1', 1, { status: 'completed' })
		expect(store.tasks.map((t) => t.seq)).toEqual([1, 2, 3])
		expect(store.activeTasks).toEqual([])

		// Several at once: a backgrounded job leaves its task running while the agent
		// starts the next one, so this must not collapse to the first.
		await store.update('s1', 2, { status: 'in_progress' })
		await store.update('s1', 3, { status: 'in_progress' })
		expect(store.activeTasks.map((t) => t.subject)).toEqual(['b', 'c'])

		// s2's seq 1 is a different task; updating it must not touch s1's. Assert on the
		// subject: s1's seq 1 is already `completed`, so a status assertion would pass
		// even if s2's task had replaced it wholesale.
		expect(await store.update('s2', 1, { subject: 'renamed in s2' })).toBeDefined()
		expect(store.tasks.find((t) => t.seq === 1)?.subject).toBe('a')
		expect(await store.update('s1', 99, { status: 'completed' })).toBeUndefined()
	})

	// Two stores over one session = the same session open in two tabs. Numbering from
	// each store's own snapshot made both pick seq 1, and since seq is half the primary
	// key the second put silently destroyed the first tab's task.
	it('does not lose a task when two stores append to one session', async () => {
		const tabA = store
		const tabB = makeStore()
		await tabA.setSession('s1')
		await tabB.setSession('s1')

		await tabA.createMany('s1', [mk('from tab A')])
		// tabB's in-memory list predates tabA's write.
		await tabB.createMany('s1', [mk('from tab B')])

		const persisted = await db.listTasksForSession('s1')
		expect(persisted.map((t) => t.subject).sort()).toEqual(['from tab A', 'from tab B'])
		expect(new Set(persisted.map((t) => t.seq)).size).toBe(2)
	})

	it('survives a reload of the same session', async () => {
		await store.setSession('s1')
		await store.createMany('s1', [mk('a'), mk('b')])
		await store.update('s1', 2, { status: 'in_progress' })

		await store.setSession(undefined)
		await store.setSession('s1')
		expect(store.tasks.map((t) => [t.seq, t.status])).toEqual([
			[1, 'pending'],
			[2, 'in_progress']
		])
	})

	it('removes a task without renumbering the rest', async () => {
		await store.setSession('s1')
		await store.createMany('s1', [mk('a'), mk('b'), mk('c')])
		await store.remove('s1', 2)
		expect(store.tasks.map((t) => t.seq)).toEqual([1, 3])

		// The next created task must not collide with the surviving seq 3.
		expect((await store.createMany('s1', [mk('d')])).map((t) => t.seq)).toEqual([4])
	})
})

describe('summarizeTasks', () => {
	const task = (seq: number, subject: string, status: any) => ({
		sessionId: 's',
		seq,
		subject,
		description: '',
		status,
		createdAt: 0,
		updatedAt: 0
	})

	it('reports progress and every running task', () => {
		expect(summarizeTasks([])).toBe('No tasks.')
		expect(
			summarizeTasks([
				task(1, 'a', 'completed'),
				task(2, 'b', 'in_progress'),
				task(3, 'c', 'pending')
			])
		).toBe('1/3 done, now: b')
		expect(summarizeTasks([task(1, 'a', 'completed')])).toBe('1/1 done')
		// Concurrent work (a detached job plus the next task) must all be named —
		// reporting only the first would understate what is in flight.
		expect(
			summarizeTasks([
				task(1, 'a', 'in_progress'),
				task(2, 'b', 'in_progress'),
				task(3, 'c', 'pending')
			])
		).toBe('0/3 done, now: a, b')
	})
})
