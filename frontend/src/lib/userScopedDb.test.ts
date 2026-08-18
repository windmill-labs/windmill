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

	it('yields its connection so another tab can upgrade the schema', async () => {
		userStore.set(asUser('a@x.com'))
		const held = userScopedDb<TestSchema>('t', { version: 1, upgrade })
		expect(await held.whenReady()).toBeDefined()

		// Second tab, higher version. Without the blocking handler the open never settles
		// and this await hangs rather than failing.
		const upgrading = userScopedDb<TestSchema>('t', { version: 2, upgrade })
		expect((await upgrading.whenReady())?.version).toBe(2)

		// The tab that yielded is still on the old schema, so its reopen cannot succeed —
		// it degrades to in-memory like any failed open, rather than hanging or throwing.
		expect(await held.whenReady()).toBeUndefined()
	})

	it('gives up on an upgrade an uncooperative connection is blocking', async () => {
		userStore.set(asUser('a@x.com'))
		// A tab running a build older than the blocking handler: it holds v1 open and never
		// hears versionchange, so nothing this side can do will make it let go.
		const legacy = await openDB<TestSchema>('t::a@x.com', 1, { upgrade })
		const dbh = userScopedDb<TestSchema>('t', { version: 2, upgrade, openGraceMs: 20 })

		// Bounded, so callers degrade to in-memory instead of awaiting it forever.
		expect(await dbh.whenReady()).toBeUndefined()

		// Every later call gives up too, rather than reopening behind the parked request.
		expect(await dbh.whenReady()).toBeUndefined()
		expect(await dbh.whenReady()).toBeUndefined()

		// Giving up is not permanent: once the blocker goes, the next call gets the DB.
		legacy.close()
		await vi.waitFor(async () => expect((await dbh.whenReady())?.version).toBe(2))
	})

	it('gives up on an open that is queued behind another and so hears nothing', async () => {
		userStore.set(asUser('a@x.com'))
		// What a second upgrading opener sees while a first one sits blocked: the browser
		// processes a database's opens in order, so this request waits its turn without
		// reaching `blocked` — or any other callback — of its own.
		let arrive!: (db: IDBPDatabase<TestSchema>) => void
		const queuedOpen = (() =>
			new Promise((resolve) => (arrive = resolve as never))) as unknown as typeof openDB

		const dbh = userScopedDb<TestSchema>('t', {
			version: 2,
			upgrade,
			openGraceMs: 20,
			openDB: queuedOpen
		})

		expect(await dbh.whenReady()).toBeUndefined()

		// Giving up did not cancel it — nothing can. When its turn comes it is adopted.
		const real = await openDB<TestSchema>('t::a@x.com', 2, { upgrade })
		arrive(real)
		await vi.waitFor(async () => expect(await dbh.whenReady()).toBe(real))
		real.close()
	})

	it('keeps degrading, not hanging, across a user switch away and back', async () => {
		userStore.set(asUser('a@x.com'))
		const legacy = await openDB<TestSchema>('t::a@x.com', 1, { upgrade })
		const dbh = userScopedDb<TestSchema>('t', { version: 2, upgrade, openGraceMs: 20 })
		expect(await dbh.whenReady()).toBeUndefined()

		// B is a different physical database, so it opens normally.
		userStore.set(asUser('b@y.com'))
		expect(await dbh.whenReady()).toBeDefined()

		// Back to A, whose open is still parked: reopening here would hang, not degrade.
		userStore.set(asUser('a@x.com'))
		expect(await dbh.whenReady()).toBeUndefined()

		legacy.close()
		await vi.waitFor(async () => expect((await dbh.whenReady())?.version).toBe(2))
	})

	it('issues one open per database however often callers give up on it', async () => {
		userStore.set(asUser('a@x.com'))
		const legacy = await openDB<TestSchema>('t::a@x.com', 1, { upgrade })
		const opens: string[] = []
		const countingOpen = ((name: string, version: number, cbs: unknown) => {
			opens.push(name)
			return openDB(name as never, version, cbs as never)
		}) as unknown as typeof openDB

		const dbh = userScopedDb<TestSchema>('t', {
			version: 2,
			upgrade,
			openGraceMs: 20,
			openDB: countingOpen
		})

		// Give up, drop the handle, switch away and back — every path that used to start over.
		expect(await dbh.whenReady()).toBeUndefined()
		expect(await dbh.whenReady()).toBeUndefined()
		dbh.close()
		expect(await dbh.whenReady()).toBeUndefined()
		userStore.set(asUser('b@y.com'))
		await dbh.whenReady()
		userStore.set(asUser('a@x.com'))
		expect(await dbh.whenReady()).toBeUndefined()

		expect(opens.filter((n) => n === 't::a@x.com')).toHaveLength(1)

		legacy.close()
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
