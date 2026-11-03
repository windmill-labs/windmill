import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { openDB } from 'idb'

// The cache hydrates from a BROWSER-gated user subscription.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

import { userStore, type UserExt } from '$lib/stores'
import { OpenAPI } from '$lib/gen'
import {
	cacheIdentity,
	cacheKeys,
	clearPersistedCache,
	loadEntry,
	MAX_AGE_MS,
	MAX_ENTRIES,
	peekEntry,
	saveEntry,
	snapshotJson,
	type CacheEntry
} from './persistedCache.svelte'
import { persistedValue, pickShown } from './persistedResource.svelte'
import { readSessionHint, rememberUser } from './lastSession'

function login(email: string, extra: Partial<UserExt> = {}) {
	userStore.set({ email, username: email.split('@')[0], ...extra } as unknown as UserExt)
}

async function stored(email: string): Promise<string[]> {
	const db = await openDB(`windmill-persisted-cache::${email}`)
	try {
		return db.objectStoreNames.contains('entries')
			? ((await db.getAllKeys('entries')) as string[])
			: []
	} finally {
		db.close()
	}
}

function keyFor(name: string, workspace: string, args?: unknown) {
	return cacheKeys(cacheIdentity()!, name, { workspace, args }).key
}

// The setup file's fake IndexedDB lasts the whole file, as the per-user database handle does:
// each test logs in users of its own.
beforeEach(() => {
	OpenAPI.TOKEN = undefined
	userStore.set(undefined)
})
afterEach(() => {
	vi.useRealTimers()
})

describe('keying', () => {
	it('has no identity, so nothing is read or written, while no one is logged in', () => {
		expect(cacheIdentity()).toBeUndefined()
	})

	it('tells users, impersonations and explicit tokens apart', () => {
		login('a@x.dev')
		const a = cacheIdentity()
		login('a@x.dev', { impersonating_email: 'root@x.dev' })
		const impersonated = cacheIdentity()
		login('a@x.dev')
		OpenAPI.TOKEN = 'tok_1'
		const t1 = cacheIdentity()
		OpenAPI.TOKEN = 'tok_2'
		const t2 = cacheIdentity()
		login('b@x.dev')
		OpenAPI.TOKEN = undefined
		const b = cacheIdentity()
		expect(new Set([a, impersonated, t1, t2, b]).size).toBe(5)
		expect(t1).not.toContain('tok_1')
	})

	it('files equal arguments under one key, whatever their order, and others apart', () => {
		const k = (ws: string, args: unknown) => cacheKeys('id', 'n', { workspace: ws, args })
		expect(k('w', { a: 1, b: undefined, c: [1, { y: 2, x: 1 }] }).key).toBe(
			k('w', { c: [1, { x: 1, y: 2 }], a: 1 }).key
		)
		expect(k('w', { a: 1 }).key).not.toBe(k('w', { a: 2 }).key)
		expect(k('w', { a: 1 }).key).not.toBe(k('w2', { a: 1 }).key)
		expect(k('w', { a: 1 }).scope).toBe(k('w', { a: 2 }).scope)
		expect(cacheKeys('id', 'n', { workspace: 'w' }).key).not.toBe(
			cacheKeys('id2', 'n', { workspace: 'w' }).key
		)
	})

	it("never shows another user's or workspace's response", () => {
		const here = { scope: 'S', key: 'S#1' }
		const otherArgs = { scope: 'S', key: 'S#2' }
		const elsewhere = { scope: 'T', key: 'T#1' }
		// Fetched for these keys wins over the cache; the cache stands in until then.
		expect(pickShown({ keys: here, value: 'fresh' }, here, 'cached')).toBe('fresh')
		expect(pickShown(undefined, here, 'cached')).toBe('cached')
		// A filter change in the same scope keeps the previous response up; nothing cached.
		expect(pickShown({ keys: otherArgs, value: 'prev' }, here, undefined)).toBe('prev')
		expect(pickShown({ keys: otherArgs, value: 'prev' }, here, 'cached')).toBe('cached')
		// Another scope's response is dropped even with nothing else to show.
		expect(pickShown({ keys: elsewhere, value: 'theirs' }, here, undefined)).toBeUndefined()
		expect(pickShown({ keys: elsewhere, value: 'theirs' }, undefined, undefined)).toBeUndefined()
	})
})

describe('versioning', () => {
	it('reads an entry of another version as none, and drops it from disk', async () => {
		login('v@x.dev')
		const key = keyFor('runs', 'w')
		saveEntry(key, 1, { shape: 'old' })
		await vi.waitFor(async () => expect(await stored('v@x.dev')).toContain(key))
		expect(peekEntry(key, 1)).toEqual({ shape: 'old' })
		expect(peekEntry(key, 2)).toBeUndefined()

		// A reload: memory is gone, the disk holds the old shape.
		login('other@x.dev')
		login('v@x.dev')
		await vi.waitFor(() => expect(peekEntry(key, 1)).toBeDefined())
		login('other@x.dev')
		login('v@x.dev')
		expect(await loadEntry(key, 2)).toBeUndefined()
		await vi.waitFor(async () => expect(await stored('v@x.dev')).not.toContain(key))
	})
})

describe('invalidation', () => {
	it("hydrates the user's entries on login, and none of another's", async () => {
		login('h1@x.dev')
		const mine = keyFor('folders', 'w')
		saveEntry(mine, 1, ['f/a'])
		await vi.waitFor(async () => expect(await stored('h1@x.dev')).toContain(mine))

		login('h2@x.dev')
		expect(peekEntry(mine, 1)).toBeUndefined()
		expect(await loadEntry(mine, 1)).toBeUndefined()
		expect(peekEntry(keyFor('folders', 'w'), 1)).toBeUndefined()

		login('h1@x.dev')
		await vi.waitFor(() => expect(peekEntry(mine, 1)).toEqual(['f/a']))
	})

	it("clears the logged-in user's entries on logout, and only theirs", async () => {
		login('keep@x.dev')
		const kept = keyFor('runs', 'w')
		saveEntry(kept, 1, 'kept')
		await vi.waitFor(async () => expect(await stored('keep@x.dev')).toContain(kept))

		login('out@x.dev')
		const gone = keyFor('runs', 'w')
		saveEntry(gone, 1, 'gone')
		await vi.waitFor(async () => expect(await stored('out@x.dev')).toContain(gone))

		// The synchronous snapshot read at boot goes with it.
		await vi.waitFor(() =>
			expect(localStorage.getItem('wm-persisted-cache-snapshot')).toContain('out@x.dev')
		)
		await clearPersistedCache()
		expect(peekEntry(gone, 1)).toBeUndefined()
		expect(localStorage.getItem('wm-persisted-cache-snapshot')).toBeNull()
		expect(await stored('out@x.dev')).toEqual([])
		expect(await stored('keep@x.dev')).toEqual([kept])
	})

	it('drops an entry past its maximum age, from disk at the next hydration', async () => {
		vi.useFakeTimers({ toFake: ['Date'] })
		login('age@x.dev')
		const key = keyFor('runs', 'w')
		saveEntry(key, 1, 'old')
		expect(peekEntry(key, 1)).toBe('old')
		await vi.waitFor(async () => expect(await stored('age@x.dev')).toContain(key))
		vi.setSystemTime(Date.now() + MAX_AGE_MS + 1)
		expect(peekEntry(key, 1)).toBeUndefined()
		// The next hydration (a reload) drops it from disk.
		login('other@x.dev')
		login('age@x.dev')
		await vi.waitFor(async () => expect(await stored('age@x.dev')).not.toContain(key))
	})

	it('evicts the oldest entries past the per-user cap', async () => {
		vi.useFakeTimers({ toFake: ['Date'] })
		login('cap@x.dev')
		const keys = Array.from({ length: MAX_ENTRIES + 2 }, (_, i) => keyFor('runs', `w${i}`))
		for (const key of keys) {
			vi.setSystemTime(Date.now() + 1000)
			saveEntry(key, 1, key)
		}
		await vi.waitFor(async () => expect(await stored('cap@x.dev')).toHaveLength(MAX_ENTRIES))
		const left = await stored('cap@x.dev')
		expect(left).not.toContain(keys[0])
		expect(left).not.toContain(keys[1])
		expect(left).toContain(keys[keys.length - 1])
	})

	it("stops reading and writing as this tab's user once another login swaps the session", async () => {
		let cookie = `${__WM_SESSION_HINT__}=alice-session`
		;(globalThis as any).document = {
			get cookie() {
				return cookie
			}
		}
		try {
			login('alice@x.dev')
			rememberUser(
				{ email: 'alice@x.dev', workspace_id: 'w' } as unknown as UserExt,
				readSessionHint()
			)
			const runs = persistedValue<string>('runs', 1)
			expect(cacheIdentity()).toBeDefined()
			runs.save({ workspace: 'w' }, 'alice rows', readSessionHint())
			expect(runs.peek({ workspace: 'w' })).toBe('alice rows')

			// Bob logs in from another tab: this tab still holds alice, but requests now carry
			// bob's cookie, so a response may be bob's.
			cookie = `${__WM_SESSION_HINT__}=bob-session`
			expect(cacheIdentity()).toBeUndefined()
			runs.save({ workspace: 'w' }, 'bob rows', readSessionHint())
			expect(runs.peek({ workspace: 'w' })).toBeUndefined()

			// Back on alice's session: her entry is untouched.
			cookie = `${__WM_SESSION_HINT__}=alice-session`
			expect(runs.peek({ workspace: 'w' })).toBe('alice rows')
		} finally {
			delete (globalThis as any).document
		}
	})

	it('keeps the snapshot within its cap by leaving out the largest entries, not all of them', () => {
		const entry = (key: string, chars: number): CacheEntry => ({
			key,
			version: 1,
			savedAt: 0,
			value: 'x'.repeat(chars)
		})
		const entries = [entry('runs', 100), entry('folders', 5000), entry('counts', 50)]
		const raw = snapshotJson('a@x.dev', entries, 1000)
		expect(raw.length).toBeLessThanOrEqual(1000)
		const kept = (JSON.parse(raw) as { email: string; entries: CacheEntry[] }).entries
		expect(kept.map((e) => e.key).sort()).toEqual(['counts', 'runs'])
		expect(JSON.parse(snapshotJson('a@x.dev', entries, 10)).entries).toEqual([])
	})
})
