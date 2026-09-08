import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import type { ArtifactVersion, PersistedArtifact } from './artifactsDB'

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

function version(v: number, artifactId = 'a1'): ArtifactVersion {
	return {
		key: `${artifactId}:${v}`,
		artifactId,
		version: v,
		name: 'Doc',
		content: `body ${v}`,
		savedAt: v
	}
}

// mutateArtifact takes a mutator, not a row: these tests only need "write exactly this".
const put = (m: any, a: any, snapshots: any[]) =>
	m.mutateArtifact(a.id, () => ({ artifact: a, snapshots }))

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

	it('upgrades a version-1 database in place, keeping the artifacts already in it', async () => {
		vi.resetModules()
		;(globalThis as any).indexedDB = new IDBFactory()
		const store = (await import('$lib/stores')).userStore as unknown as {
			set: (v: unknown) => void
		}
		store.set({ email: 'a@x.com' })

		// The schema exactly as it shipped before version history: `items` and nothing else.
		// Every existing user's database looks like this, and the upgrade runs over it — the
		// path no other test reaches, because they all start from an empty IDBFactory.
		const { openDB } = await import('idb')
		const v1 = (await openDB('copilot-artifacts::a@x.com', 1, {
			upgrade(database) {
				const items = (database as any).createObjectStore('items', { keyPath: 'id' })
				items.createIndex('by-session', 'sessionId')
			}
		})) as any
		await v1.put('items', artifact({ id: 'old', sessionId: 's1', content: 'written at v1' }))
		v1.close()

		const upgraded = await import('./artifactsDB')
		// A ConstraintError here would reject the open, and a rejected open degrades silently
		// — every pre-existing artifact would just quietly stop existing.
		expect((await upgraded.getArtifact('old'))?.content).toBe('written at v1')
		expect((await upgraded.listArtifactsForSession('s1')).map((a) => a.id)).toEqual(['old'])
		// The store the upgrade added works on the upgraded database, not just a fresh one.
		await put(upgraded, artifact({ id: 'old' }), [version(1, 'old')])
		expect((await upgraded.listArtifactVersions('old')).map((v) => v.version)).toEqual([1])
	})

	it('keeps only the most recent versions of an artifact', async () => {
		const total = db.MAX_VERSIONS_PER_ARTIFACT + 5
		for (let v = 1; v <= total; v++) await put(db, artifact({ id: 'a1' }), [version(v)])

		const kept = await db.listArtifactVersions('a1')
		expect(kept).toHaveLength(db.MAX_VERSIONS_PER_ARTIFACT)
		// Newest first, and the pruned tail is the numerically — not lexicographically —
		// oldest, which is what separates v9 from v10 surviving.
		expect(kept[0].version).toBe(total)
		expect(kept.at(-1)?.version).toBe(total - db.MAX_VERSIONS_PER_ARTIFACT + 1)
	})

	it('never prunes away the version that stands as the approved plan', async () => {
		// Approve at v1, then plan against it for another twenty rounds: without this the
		// text the user agreed to is the first thing the ring buffer drops.
		const total = db.MAX_VERSIONS_PER_ARTIFACT + 5
		for (let v = 1; v <= total; v++) {
			await put(db, artifact({ id: 'a1', role: 'plan', approvedVersion: 1 }), [version(v)])
		}

		const kept = await db.listArtifactVersions('a1')
		// Protected, not extra: the budget is unchanged, so the survivors are v1 plus the
		// newest MAX-1 rather than a contiguous run.
		expect(kept).toHaveLength(db.MAX_VERSIONS_PER_ARTIFACT)
		expect(kept.at(-1)?.version).toBe(1)
		expect(kept.at(-2)?.version).toBe(total - db.MAX_VERSIONS_PER_ARTIFACT + 2)
	})

	it('keeps fewer versions of a large artifact, but never fewer than the minimum', async () => {
		// Big enough that the char budget, not the count, decides — a plain count cap would
		// let one document's history run to several MB.
		const big = 'x'.repeat(db.MAX_VERSION_CHARS_PER_ARTIFACT / 4)
		for (let v = 1; v <= 8; v++)
			await put(db, artifact({ id: 'a1' }), [{ ...version(v), content: big }])

		const kept = await db.listArtifactVersions('a1')
		expect(kept).toHaveLength(4)
		expect(kept[0].version).toBe(8)

		// A single snapshot larger than the whole budget still leaves a usable history.
		const huge = 'x'.repeat(db.MAX_VERSION_CHARS_PER_ARTIFACT * 2)
		await put(db, artifact({ id: 'a1' }), [{ ...version(9), content: huge }])
		expect(await db.listArtifactVersions('a1')).toHaveLength(db.MIN_VERSIONS_PER_ARTIFACT)
	})

	it('deleting an artifact, or a whole session, drops the versions with it', async () => {
		await db.putArtifact(artifact({ id: 'a1', sessionId: 's1' }))
		await db.putArtifact(artifact({ id: 'a2', sessionId: 's1' }))
		await put(db, artifact({ id: 'a1', sessionId: 's1' }), [version(1)])
		await put(db, artifact({ id: 'a2', sessionId: 's1' }), [version(1, 'a2')])

		await db.deleteArtifact('a1')
		expect(await db.listArtifactVersions('a1')).toEqual([])
		expect(await db.listArtifactVersions('a2')).toHaveLength(1)

		await db.deleteArtifactsForSession('s1')
		expect(await db.listArtifactVersions('a2')).toEqual([])
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

	it('rejects a version read it could not make, instead of reading as absent', async () => {
		// The one read here that does not degrade to undefined: a caller clears a reader's pinned
		// version on absence, and a pin cleared over a transient failure is cleared for good.
		vi.resetModules()
		delete (globalThis as any).indexedDB
		;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
		const noDb = await import('./artifactsDB')
		await expect(noDb.getArtifactVersion('a1', 2)).rejects.toThrow(/unavailable/)

		// And with a handle in hand, whose get rejects (a force-closed connection).
		vi.resetModules()
		vi.doMock('idb', () => ({
			openDB: async () => ({
				get: async () => {
					throw new DOMException('closed', 'InvalidStateError')
				}
			}),
			deleteDB: async () => {}
		}))
		try {
			;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
			const failing = await import('./artifactsDB')
			await expect(failing.getArtifactVersion('a1', 2)).rejects.toThrow(/closed/)
		} finally {
			vi.doUnmock('idb')
		}
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
