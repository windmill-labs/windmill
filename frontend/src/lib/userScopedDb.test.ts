import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { openDB, type DBSchema, type IDBPDatabase } from 'idb'

// scopedKey resolves the email from userStore via a BROWSER-gated subscription.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

import { userStore, type UserExt } from '$lib/stores'
import { userScopedDb, type UserScopedDbMigrateDeps } from './userScopedDb'

interface TestSchema extends DBSchema {
	items: { key: string; value: { id: string; v: number } }
}

function upgrade(db: IDBPDatabase<TestSchema>) {
	if (!db.objectStoreNames.contains('items')) {
		db.createObjectStore('items', { keyPath: 'id' })
	}
}

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}

beforeEach(() => {
	// Fresh in-memory IndexedDB per test for isolation.
	;(globalThis as any).indexedDB = new IDBFactory()
	userStore.set(undefined)
})

describe('userScopedDb', () => {
	it('returns undefined while no user is logged in', async () => {
		const dbh = userScopedDb<TestSchema>('t', { version: 1, upgrade })
		expect(await dbh.whenReady()).toBeUndefined()
	})

	it('isolates data between users and restores it on return', async () => {
		const dbh = userScopedDb<TestSchema>('t', { version: 1, upgrade })

		userStore.set(asUser('a@x.com'))
		const dbA = await dbh.whenReady()
		await dbA!.put('items', { id: 'i1', v: 1 })

		// Switch user: whenReady reopens the other user's (empty) DB — A's record
		// is not visible.
		userStore.set(asUser('b@y.com'))
		const dbB = await dbh.whenReady()
		expect(await dbB!.count('items')).toBe(0)
		await dbB!.put('items', { id: 'i2', v: 2 })

		// Back to A: their record is intact, B's is not present.
		userStore.set(asUser('a@x.com'))
		const dbA2 = await dbh.whenReady()
		expect((await dbA2!.getAll('items')).map((x) => x.id)).toEqual(['i1'])
	})

	it('runs migrate once per scoped name and claims+deletes the legacy DB', async () => {
		// Seed a legacy (un-namespaced) DB, mirroring the chat-history pattern.
		const legacy = await openDB<TestSchema>('t', 1, { upgrade })
		await legacy.put('items', { id: 'legacy1', v: 9 })
		legacy.close()

		const migrate = vi.fn(async (db: IDBPDatabase<TestSchema>, deps: UserScopedDbMigrateDeps) => {
			if ((await db.count('items')) > 0) return
			const src = await deps.openDB<TestSchema>('t', 1, { upgrade })
			const all = await src.getAll('items')
			const tx = db.transaction('items', 'readwrite')
			await Promise.all([...all.map((x) => tx.store.put(x)), tx.done])
			src.close()
			await deps.deleteDB('t')
		})

		const dbh = userScopedDb<TestSchema>('t', { version: 1, upgrade, migrate })
		userStore.set(asUser('a@x.com'))

		const db = await dbh.whenReady()
		expect((await db!.getAll('items')).map((x) => x.id)).toEqual(['legacy1'])
		// Legacy bare DB was deleted.
		const names = (await indexedDB.databases()).map((d) => d.name)
		expect(names).not.toContain('t')
		expect(names).toContain('t::a@x.com')

		// migrate is gated to once per scoped name even across repeated whenReady.
		await dbh.whenReady()
		expect(migrate).toHaveBeenCalledTimes(1)
	})

	it('degrades to undefined (no throw) when the DB cannot be opened', async () => {
		const failingOpen = vi.fn(async () => {
			throw new Error('blocked')
		}) as unknown as typeof openDB
		const dbh = userScopedDb<TestSchema>('t', { version: 1, upgrade, openDB: failingOpen })
		userStore.set(asUser('a@x.com'))
		expect(await dbh.whenReady()).toBeUndefined()
	})

	it('clears the handle on logout', async () => {
		const dbh = userScopedDb<TestSchema>('t', { version: 1, upgrade })
		userStore.set(asUser('a@x.com'))
		expect(await dbh.whenReady()).toBeDefined()
		userStore.set(undefined)
		expect(await dbh.whenReady()).toBeUndefined()
	})
})
