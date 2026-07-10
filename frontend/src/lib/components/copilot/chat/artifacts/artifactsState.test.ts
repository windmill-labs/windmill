import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { SessionArtifactsStore } from './artifactsState.svelte'
import * as db from './artifactsDB'

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
	const dbMod = await import('./artifactsDB')
	const { SessionArtifactsStore: Store } = await import('./artifactsState.svelte')
	return { dbMod, store: new Store() }
}

let store: SessionArtifactsStore
let dbMod: typeof db
beforeEach(async () => {
	;({ store, dbMod } = await fresh())
})

describe('SessionArtifactsStore', () => {
	it('loads the current session, newest-updated first', async () => {
		await dbMod.putArtifact(mk({ id: 'old', sessionId: 's1', updatedAt: 1 }))
		await dbMod.putArtifact(mk({ id: 'new', sessionId: 's1', updatedAt: 2 }))
		await dbMod.putArtifact(mk({ id: 'other', sessionId: 's2', updatedAt: 3 }))

		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.id)).toEqual(['new', 'old'])
		expect(store.loading).toBe(false)
	})

	it('empties the list for an undefined session id', async () => {
		await dbMod.putArtifact(mk({ id: 'a', sessionId: 's1' }))
		await store.setSession('s1')
		expect(store.artifacts).toHaveLength(1)

		await store.setSession(undefined)
		expect(store.artifacts).toEqual([])
	})

	it('a later setSession wins over an earlier in-flight load', async () => {
		await dbMod.putArtifact(mk({ id: 'a', sessionId: 's1' }))
		await dbMod.putArtifact(mk({ id: 'b', sessionId: 's2' }))

		// Start both loads without awaiting the first; the last-started must win.
		const first = store.setSession('s1')
		const second = store.setSession('s2')
		await Promise.all([first, second])
		expect(store.artifacts.map((a) => a.id)).toEqual(['b'])
	})

	it('a create is not clobbered by an in-flight load with a stale snapshot', async () => {
		// Hold the load open with a stale (empty) snapshot until after the create lands.
		let releaseLoad!: (items: db.PersistedArtifact[]) => void
		const held = new Promise<db.PersistedArtifact[]>((r) => (releaseLoad = r))
		const spy = vi.spyOn(dbMod, 'listArtifactsForSession').mockReturnValueOnce(held)

		const loading = store.setSession('s1') // #load starts, hangs on `held`
		const created = await store.create('s1', { name: 'X', content: 'x' })
		releaseLoad([]) // the load resolves late, snapshot predates the create
		await loading
		spy.mockRestore()

		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// The superseded load early-returns; the create must have cleared `loading`.
		expect(store.loading).toBe(false)
	})

	it('create persists and prepends when the session is loaded', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: '# hi' })

		expect(created.kind).toBe('md')
		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// Survives a reload from the DB.
		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.name)).toEqual(['Plan'])
	})

	it('create stamps the provenance chatId when supplied', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'x', chatId: 'c9' })
		expect(created.chatId).toBe('c9')
	})

	it('create persists to another session without touching the loaded list', async () => {
		await store.setSession('s1')
		const created = await store.create('s2', { name: 'Elsewhere', content: 'x' })
		expect(store.artifacts).toEqual([])
		expect((await dbMod.listArtifactsForSession('s2')).map((a) => a.id)).toEqual([created.id])
	})

	it('update merges changes, bumps updatedAt, and persists', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'v1' })

		const updated = await store.update(created.id, { content: 'v2' })
		expect(updated?.content).toBe('v2')
		expect(updated?.name).toBe('Plan')
		expect(updated!.updatedAt).toBeGreaterThanOrEqual(created.updatedAt)
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v2')
	})

	it('update falls back to the DB when the target is not in the loaded list', async () => {
		const created = await store.create('s2', { name: 'Off', content: 'v1' })
		await store.setSession('s1') // s2 is not loaded

		const updated = await store.update(created.id, { name: 'Renamed' })
		expect(updated?.name).toBe('Renamed')
		expect(updated?.content).toBe('v1')
	})

	it('update returns undefined for an unknown id', async () => {
		await store.setSession('s1')
		expect(await store.update('nope', { content: 'x' })).toBeUndefined()
	})

	it('get resolves from the in-memory list even when the DB lacks the record', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })
		// Simulate a persist that never landed: drop it from the DB, keep it in memory.
		await dbMod.deleteArtifact(created.id)
		expect((await store.get(created.id))?.id).toBe(created.id)
	})

	it('get falls back to the DB for an artifact outside the loaded list', async () => {
		const created = await store.create('s2', { name: 'B', content: 'y' })
		await store.setSession('s1') // s2 not loaded
		expect((await store.get(created.id))?.id).toBe(created.id)
		expect(await store.get('nope')).toBeUndefined()
	})

	it('listForSession returns the in-memory list for the loaded session, even when the DB lacks it', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })
		await dbMod.deleteArtifact(created.id) // persist "failed": gone from DB, kept in memory
		expect((await store.listForSession('s1')).map((a) => a.id)).toEqual([created.id])
	})

	it('listForSession reads the DB for a session that is not loaded', async () => {
		await store.create('s2', { name: 'B', content: 'y' })
		await store.setSession('s1') // s2 not loaded
		expect((await store.listForSession('s2')).map((a) => a.name)).toEqual(['B'])
		expect(await store.listForSession('s1')).toEqual([])
	})

	it('update refuses an artifact from a different session when scoped by sessionId', async () => {
		const created = await store.create('s2', { name: 'Off', content: 'v1' })
		expect(await store.update(created.id, { content: 'v2' }, { sessionId: 's1' })).toBeUndefined()
		// Unchanged in the DB.
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v1')
		// Correct scope still updates.
		expect(await store.update(created.id, { content: 'v2' }, { sessionId: 's2' })).toBeDefined()
	})

	it('update distinguishes an omitted field from an explicit empty string', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'body' })

		// Omitting content leaves it untouched...
		expect((await store.update(created.id, { name: 'Renamed' }))?.content).toBe('body')
		// ...but an explicit empty string blanks it.
		expect((await store.update(created.id, { content: '' }))?.content).toBe('')
	})

	it('remove deletes from the DB and the loaded list', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'x' })

		await store.remove(created.id)
		expect(store.artifacts).toEqual([])
		expect(await dbMod.getArtifact(created.id)).toBeUndefined()
	})
})

function mk(over: Partial<db.PersistedArtifact> = {}): db.PersistedArtifact {
	return {
		id: 'a1',
		sessionId: 's1',
		chatId: 'c1',
		kind: 'md',
		name: 'Doc',
		content: '# hi',
		createdAt: 0,
		updatedAt: 0,
		...over
	}
}
