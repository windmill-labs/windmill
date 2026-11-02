import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { SvelteMap } from 'svelte/reactivity'
import { openDB, type DBSchema, type IDBPDatabase } from 'idb'
import { OpenAPI } from '$lib/gen'
import { userStore } from '$lib/stores'
import { userScopedDb } from '$lib/userScopedDb'
import { getCurrentUserEmail, onUserChange, scopedKeyFor } from '$lib/userScopedStorage'
import { rememberedEmail } from '$lib/lastSession'

// Last-known API responses, kept so a page can paint them before its requests return. Every
// row in them was filtered by row-level security for one user, so an entry is only ever served
// back to the identity that fetched it: the database is per user (userScopedDb), and the key
// names the user, the token and the workspace on top of that, so the in-memory layer, which
// outlives an in-place account switch, is no exception.
//
// Opt-in per call site, and only for responses holding no secret values: nothing here is
// encrypted, and an entry stays on disk until logout, eviction by newer writes, or its expiry
// (dropped at the next hydration). One of another version is never shown and is overwritten
// by the fresh response saved under the same key.

export interface CacheEntry {
	key: string
	/** The call site's format version: a response whose shape changed reads as no entry. */
	version: number
	savedAt: number
	value: unknown
}

interface CacheSchema extends DBSchema {
	entries: { key: string; value: CacheEntry; indexes: { savedAt: number } }
}

/** Entries past this are dropped rather than shown: a page left alone that long has moved on. */
export const MAX_AGE_MS = 7 * 24 * 3600 * 1000
/** Per user. A write past it evicts the oldest entries. */
export const MAX_ENTRIES = 64

const DB_NAME = 'windmill-persisted-cache'
const DB_VERSION = 1
function upgrade(database: IDBPDatabase<CacheSchema>) {
	if (!database.objectStoreNames.contains('entries')) {
		database.createObjectStore('entries', { keyPath: 'key' }).createIndex('savedAt', 'savedAt')
	}
}

const db = userScopedDb<CacheSchema>(DB_NAME, { version: DB_VERSION, upgrade })

const memory = new SvelteMap<string, CacheEntry>()
// Read by cacheIdentity so a key derived in a component follows an in-place account switch.
let userTick = $state(0)

/** Not a credential: only tells two explicit tokens of one user apart. */
function fingerprint(s: string): string {
	let h = 0x811c9dc5
	for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193)
	return (h >>> 0).toString(36)
}

/**
 * Who the API answers as, or undefined when no one is logged in (then nothing is read or
 * written). An impersonation, or a bearer token set over the session cookie, can see other
 * rows than the same email's own session, so each is an identity of its own.
 */
export function cacheIdentity(): string | undefined {
	void userTick
	const email = getCurrentUserEmail()
	if (!email) return undefined
	const impersonator = get(userStore)?.impersonating_email
	const token = typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN ? OpenAPI.TOKEN : undefined
	return [email, impersonator ?? '', token ? fingerprint(token) : ''].join('|')
}

/** JSON with object keys sorted and undefined fields dropped, so equal arguments key equally. */
export function stableStringify(v: unknown): string {
	if (v === undefined) return 'null'
	if (v === null || typeof v !== 'object') return JSON.stringify(v)
	if (Array.isArray(v)) return `[${v.map(stableStringify).join(',')}]`
	const o = v as Record<string, unknown>
	return `{${Object.keys(o)
		.filter((k) => o[k] !== undefined)
		.sort()
		.map((k) => `${JSON.stringify(k)}:${stableStringify(o[k])}`)
		.join(',')}}`
}

export interface CacheScope {
	workspace: string
	/** Every argument the response depends on beyond the workspace. */
	args?: unknown
}

/**
 * The scope part (identity, name, workspace) and the full key. The scope part is what a page
 * may keep showing while one of its arguments changes; the full key is what an entry is filed
 * under.
 */
export function cacheKeys(
	identity: string,
	name: string,
	scope: CacheScope
): { scope: string; key: string } {
	const s = stableStringify([identity, name, scope.workspace])
	return { scope: s, key: `${s}#${stableStringify(scope.args ?? null)}` }
}

function usable(entry: CacheEntry | undefined, version: number, now = Date.now()) {
	return entry != undefined && entry.version === version && now - entry.savedAt <= MAX_AGE_MS
}

/** What memory holds for `key`, if it is of `version` and recent. Reactive. */
export function peekEntry<T>(key: string, version: number): T | undefined {
	const entry = memory.get(key)
	return usable(entry, version) ? (entry!.value as T) : undefined
}

/** Reads `key` from disk into memory when memory has nothing for it (a reload before hydration). */
export async function loadEntry<T>(key: string, version: number): Promise<T | undefined> {
	if (memory.has(key)) return peekEntry<T>(key, version)
	const identity = cacheIdentity()
	const handle = await db.whenReady()
	if (!handle) return undefined
	let entry: CacheEntry | undefined
	try {
		entry = await handle.get('entries', key)
	} catch {
		return undefined
	}
	// The user changed while this read was in flight: the key names the old one, so the entry
	// can never be peeked, but it would still sit in memory.
	if (!entry || cacheIdentity() !== identity) return undefined
	if (!usable(entry, version)) {
		void handle.delete('entries', key).catch(() => {})
		return undefined
	}
	if (!memory.has(key)) memory.set(key, entry)
	return peekEntry<T>(key, version)
}

export function saveEntry(key: string, version: number, value: unknown): void {
	const entry: CacheEntry = { key, version, savedAt: Date.now(), value }
	memory.set(key, entry)
	void (async () => {
		const handle = await db.whenReady()
		if (!handle) return
		try {
			const tx = handle.transaction('entries', 'readwrite')
			await tx.store.put($state.snapshot(entry) as CacheEntry)
			let excess = (await tx.store.count()) - MAX_ENTRIES
			if (excess > 0) {
				let cursor = await tx.store.index('savedAt').openCursor()
				// Memory keeps them: it lasts one page load, and dropping one there could pull
				// a value from under a page still painting it.
				while (cursor && excess-- > 0) {
					await cursor.delete()
					cursor = await cursor.continue()
				}
			}
			await tx.done
		} catch (e) {
			// Quota, a closed handle, a value that does not clone: the page works as without a cache.
			console.warn('persistedCache: could not save', e)
		}
	})()
}

async function hydrate() {
	const identity = cacheIdentity()
	const handle = await db.whenReady()
	if (!handle || !identity) return
	try {
		const entries = await handle.getAll('entries')
		if (cacheIdentity() !== identity) return
		await keep(entries, handle)
	} catch (e) {
		console.warn('persistedCache: could not read entries', e)
	}
}

/**
 * Reads the store of the user the browser's session still names (`lastSession`) while the app
 * boots, before the layout knows who is logged in, so the entries are in memory by the time
 * the page mounts. Memory keys carry the identity, so nothing read here is shown to anyone else.
 */
async function hydrateAhead(email: string) {
	let handle: IDBPDatabase<CacheSchema> | undefined
	try {
		// Same version and upgrade as `db`: an open at the version without its upgrade would
		// leave a new database with no store.
		handle = await openDB<CacheSchema>(scopedKeyFor(DB_NAME, email), DB_VERSION, { upgrade })
		const entries = await handle.getAll('entries')
		const current = getCurrentUserEmail()
		if (current === undefined || current === email) await keep(entries)
	} catch (e) {
		console.warn('persistedCache: could not read entries ahead', e)
	} finally {
		handle?.close()
	}
}

async function keep(entries: CacheEntry[], handle?: IDBPDatabase<CacheSchema>) {
	const now = Date.now()
	const expired: string[] = []
	for (const entry of entries) {
		// The version is the call site's to check on read; only age is known here.
		if (now - entry.savedAt > MAX_AGE_MS) expired.push(entry.key)
		else if (!memory.has(entry.key)) memory.set(entry.key, entry)
	}
	if (expired.length && handle) {
		const tx = handle.transaction('entries', 'readwrite')
		await Promise.all([...expired.map((k) => tx.store.delete(k)), tx.done])
	}
}

/**
 * Forgets everything cached for the user logged in now, on disk and in memory. Called on
 * logout before the user is cleared, since the database is found by their email.
 */
export async function clearPersistedCache(): Promise<void> {
	memory.clear()
	const handle = await db.whenReady()
	if (!handle) return
	try {
		await handle.clear('entries')
	} catch (e) {
		console.warn('persistedCache: could not clear', e)
	}
}

if (BROWSER) {
	const ahead = rememberedEmail()
	if (ahead) void hydrateAhead(ahead)
	// Loads the whole (bounded) store into memory once the user is known, and keeps only what
	// belongs to them: the entries read ahead for the same identity survive.
	onUserChange(() => {
		userTick++
		const identity = cacheIdentity()
		const mine = identity ? `[${JSON.stringify(identity)},` : undefined
		for (const key of [...memory.keys()]) if (!mine || !key.startsWith(mine)) memory.delete(key)
		void hydrate()
	})
}
