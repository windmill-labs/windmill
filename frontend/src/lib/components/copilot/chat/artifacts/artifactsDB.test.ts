import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import type { PersistedArtifact } from './artifactsDB'

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

function artifact(over: Partial<PersistedArtifact> = {}): PersistedArtifact {
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

// artifactsDB memoises its DB handle at module scope. Reset the module and install a
// fresh IDBFactory before each test, then import through this helper so every test
// opens its own empty database. The DB is namespaced by email, so seed a user.
let userStore: { set: (v: unknown) => void }
async function freshDb() {
	vi.resetModules()
	;(globalThis as any).indexedDB = new IDBFactory()
	userStore = (await import('$lib/stores')).userStore as never
	userStore.set({ email: 'a@x.com' })
	return await import('./artifactsDB')
}

let db: Awaited<ReturnType<typeof freshDb>>
beforeEach(async () => {
	db = await freshDb()
})

describe('artifactsDB', () => {
	it('derives filename and mime type from the artifact kind', () => {
		expect(db.artifactFilename({ name: 'Plan', kind: 'md' })).toBe('Plan.md')
		expect(db.artifactFilename({ name: 'Page', kind: 'html' })).toBe('Page.html')
		expect(db.artifactMimeType('md')).toBe('text/markdown')
		expect(db.artifactMimeType('html')).toBe('text/html')
	})

	it('round-trips an artifact through put/get', async () => {
		await db.putArtifact(artifact({ id: 'x', name: 'Plan', content: 'body' }))
		expect(await db.getArtifact('x')).toMatchObject({ id: 'x', name: 'Plan', content: 'body' })
	})

	it('get returns undefined for a missing id', async () => {
		expect(await db.getArtifact('nope')).toBeUndefined()
	})

	it('put overwrites an existing record by id', async () => {
		await db.putArtifact(artifact({ id: 'x', content: 'v1', updatedAt: 1 }))
		await db.putArtifact(artifact({ id: 'x', content: 'v2', updatedAt: 2 }))
		expect(await db.getArtifact('x')).toMatchObject({ content: 'v2', updatedAt: 2 })
		expect(await db.listArtifactsForSession('s1')).toHaveLength(1)
	})

	it('lists only the requested session, and returns [] for an unknown one', async () => {
		await db.putArtifact(artifact({ id: 'a', sessionId: 's1' }))
		await db.putArtifact(artifact({ id: 'b', sessionId: 's1' }))
		await db.putArtifact(artifact({ id: 'c', sessionId: 's2' }))
		expect((await db.listArtifactsForSession('s1')).map((a) => a.id).sort()).toEqual(['a', 'b'])
		expect(await db.listArtifactsForSession('missing')).toEqual([])
	})

	it('deletes a single artifact', async () => {
		await db.putArtifact(artifact({ id: 'a' }))
		await db.deleteArtifact('a')
		expect(await db.getArtifact('a')).toBeUndefined()
	})

	it('deletes every artifact for a session, leaving others intact', async () => {
		await db.putArtifact(artifact({ id: 'a', sessionId: 's1' }))
		await db.putArtifact(artifact({ id: 'b', sessionId: 's1' }))
		await db.putArtifact(artifact({ id: 'c', sessionId: 's2' }))
		await db.deleteArtifactsForSession('s1')
		expect(await db.listArtifactsForSession('s1')).toEqual([])
		expect((await db.listArtifactsForSession('s2')).map((a) => a.id)).toEqual(['c'])
	})

	it('isolates artifacts between users on the same browser', async () => {
		await db.putArtifact(artifact({ id: 'a', sessionId: 's1' }))
		// A different user sees an empty, separate database...
		userStore.set({ email: 'b@x.com' })
		expect(await db.listArtifactsForSession('s1')).toEqual([])
		await db.putArtifact(artifact({ id: 'b', sessionId: 's1' }))
		// ...and switching back reveals only the first user's artifact.
		userStore.set({ email: 'a@x.com' })
		expect((await db.listArtifactsForSession('s1')).map((x) => x.id)).toEqual(['a'])
	})

	it('degrades gracefully when IndexedDB is unavailable', async () => {
		vi.resetModules()
		delete (globalThis as any).indexedDB
		;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
		const noDb = await import('./artifactsDB')
		// Reads return empty; writes/deletes are no-ops rather than throwing.
		await expect(noDb.putArtifact(artifact())).resolves.toBeUndefined()
		expect(await noDb.getArtifact('a1')).toBeUndefined()
		expect(await noDb.listArtifactsForSession('s1')).toEqual([])
		await expect(noDb.deleteArtifact('a1')).resolves.toBeUndefined()
		await expect(noDb.deleteArtifactsForSession('s1')).resolves.toBeUndefined()
	})

	it('swallows a write failure when the DB handle exists but the op rejects', async () => {
		// The handle is present, but put/delete reject — the QuotaExceededError-shaped failure
		// the plain no-handle test can't reach. Must not throw at the caller.
		vi.resetModules()
		vi.doMock('idb', () => ({
			openDB: async () => ({
				put: async () => {
					throw new DOMException('quota', 'QuotaExceededError')
				},
				delete: async () => {
					throw new DOMException('quota', 'QuotaExceededError')
				}
			}),
			deleteDB: async () => {}
		}))
		try {
			;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
			const failing = await import('./artifactsDB')
			await expect(failing.putArtifact(artifact())).resolves.toBeUndefined()
			await expect(failing.deleteArtifact('a1')).resolves.toBeUndefined()
		} finally {
			vi.doUnmock('idb')
		}
	})
})
