import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { SvelteMap } from 'svelte/reactivity'
import type { DBSchema, IDBPDatabase } from 'idb'
import { OpenAPI } from '$lib/gen'
import { userStore } from '$lib/stores'
import { userScopedDb } from '$lib/userScopedDb'
import { getCurrentUserEmail, onUserChange } from '$lib/userScopedStorage'
import {
	onSessionConfirmationChange,
	rememberedEmail,
	sessionStillConfirmed
} from '$lib/lastSession'

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
	// The user is this tab's, the cookie the browser's: after another tab's login they differ
	// until this tab's next whoami, and a response then belongs to the other session.
	if (!email || !sessionStillConfirmed()) return undefined
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
	recordInSnapshot(entry)
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

// The entries last saved for one workspace, mirrored to localStorage, because IndexedDB answers
// in a task of its own, which lands behind the page's first render however early it is asked:
// read synchronously as this module loads, they are in memory for the frame that mounts the
// page. Only for the user the browser's session still names (`lastSession`); memory keys carry
// the identity, so nothing read here is shown to anyone else.
const SNAPSHOT_KEY = 'wm-persisted-cache-snapshot'
const SNAPSHOT_MAX_CHARS = 256 * 1024
let snapshot: { email: string; workspace: string; entries: Map<string, CacheEntry> } | undefined
let snapshotTimer: ReturnType<typeof setTimeout> | undefined

function workspaceOfKey(key: string): string | undefined {
	try {
		return JSON.parse(key.slice(0, key.indexOf('#')))[2]
	} catch {
		return undefined
	}
}

function recordInSnapshot(entry: CacheEntry) {
	const email = getCurrentUserEmail()
	const workspace = workspaceOfKey(entry.key)
	if (!email || !workspace) return
	if (snapshot?.email !== email || snapshot.workspace !== workspace)
		snapshot = { email, workspace, entries: new Map() }
	snapshot.entries.set(entry.key, $state.snapshot(entry) as CacheEntry)
	snapshotTimer ??= setTimeout(() => {
		snapshotTimer = undefined
		try {
			if (snapshot)
				localStorage.setItem(SNAPSHOT_KEY, snapshotJson(snapshot.email, snapshot.entries.values()))
		} catch {}
	}, 0)
}

/**
 * The snapshot as stored, within `max` characters: the smallest entries first, so one oversized
 * response (a workspace's thousands of folder names) costs only itself the first-frame paint,
 * not the counts and runs beside it. Exported for its test.
 */
export function snapshotJson(
	email: string,
	entries: Iterable<CacheEntry>,
	max = SNAPSHOT_MAX_CHARS
): string {
	const head = `{"email":${JSON.stringify(email)},"entries":[`
	let size = head.length + 2
	const kept: string[] = []
	for (const json of [...entries]
		.map((e) => JSON.stringify(e))
		.sort((a, b) => a.length - b.length)) {
		if (size + json.length + 1 > max) break
		kept.push(json)
		size += json.length + 1
	}
	return `${head}${kept.join(',')}]}`
}

function readSnapshot() {
	try {
		const email = rememberedEmail()
		const raw = localStorage.getItem(SNAPSHOT_KEY)
		if (!email || !raw) return
		const read = JSON.parse(raw) as { email: string; entries: CacheEntry[] }
		if (read.email === email) void keep(read.entries)
	} catch {}
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
	snapshot = undefined
	try {
		localStorage.removeItem(SNAPSHOT_KEY)
	} catch {}
	const handle = await db.whenReady()
	if (!handle) return
	try {
		await handle.clear('entries')
	} catch (e) {
		console.warn('persistedCache: could not clear', e)
	}
}

if (BROWSER) {
	readSnapshot()
	// Loads the whole (bounded) store into memory once the user is known, and keeps only what
	// belongs to them: the snapshot's entries for the same identity survive.
	// The identity is known once both the user and the session they hold are: either can come
	// second, and a key derived before must follow.
	const onIdentityChange = () => {
		userTick++
		// No user yet (the call at registration, as the app boots): nothing can be read without
		// an identity, and the snapshot just loaded is waiting for its user. Logout clears memory
		// itself (clearPersistedCache).
		const identity = cacheIdentity()
		if (!identity) return
		const mine = `[${JSON.stringify(identity)},`
		for (const key of [...memory.keys()]) if (!key.startsWith(mine)) memory.delete(key)
		void hydrate()
	}
	onUserChange(onIdentityChange)
	onSessionConfirmationChange(onIdentityChange)
}
