import { openDB as idbOpenDB, deleteDB as idbDeleteDB, type DBSchema, type IDBPDatabase } from 'idb'
import { scopedKey } from '$lib/userScopedStorage'

// Per-user IndexedDB lifecycle, shared by the session list and the copilot
// chat-history stores. The effective DB name is the base name namespaced by the
// logged-in user's email (scopedKey), so two users on a shared browser never
// touch the same physical database.
//
// `whenReady()` is name-aware: it computes the current scoped name on every call
// and transparently closes + reopens when the email changes, so the handle
// self-heals on user switch WITHOUT subscribing to onUserChange. That matters
// because there is one handle per HistoryManager instance (singleton + one per
// session runtime) — a per-instance subscription would leak callbacks.

export interface UserScopedDbMigrateDeps {
	openDB: typeof idbOpenDB
	deleteDB: typeof idbDeleteDB
}

export interface UserScopedDbOptions<Schema extends DBSchema> {
	version: number
	upgrade: (db: IDBPDatabase<Schema>) => void
	// Invoked once per scoped name right after a successful open. The fn owns its
	// own "already migrated / not applicable" gate (e.g. checking a store's
	// count) — claim-then-delete legacy data lives here.
	migrate?: (db: IDBPDatabase<Schema>, deps: UserScopedDbMigrateDeps) => Promise<void>
	// Injectable for tests (defaults to the real idb implementations).
	openDB?: typeof idbOpenDB
	deleteDB?: typeof idbDeleteDB
	// How long a blocked upgrade waits before giving up. Injectable for tests, which
	// cannot afford the real grace period.
	blockedGraceMs?: number
}

export interface UserScopedDb<Schema extends DBSchema> {
	// Resolves to the open DB for the current user, or undefined when no user is logged
	// in yet, the open failed, or another connection is holding up a schema upgrade
	// (degrade to in-memory; never rejects, never hangs).
	whenReady(): Promise<IDBPDatabase<Schema> | undefined>
	close(): void
}

// How long a blocked upgrade waits before the opener gives up. Only a connection from a
// build predating the blocking handler below can block one (it never hears versionchange),
// so this is a rollout-window escape hatch, not a routine path.
const BLOCKED_OPEN_GRACE_MS = 5000

export function userScopedDb<Schema extends DBSchema>(
	baseName: string,
	opts: UserScopedDbOptions<Schema>
): UserScopedDb<Schema> {
	const openDB = opts.openDB ?? idbOpenDB
	const deleteDB = opts.deleteDB ?? idbDeleteDB
	const blockedGraceMs = opts.blockedGraceMs ?? BLOCKED_OPEN_GRACE_MS
	const migratedNames = new Set<string>()

	let openName: string | undefined
	let openPromise: Promise<IDBPDatabase<Schema> | undefined> | undefined
	// Set once openPromise settles, so a resolved handle is handed out without re-racing.
	let settled: { db: IDBPDatabase<Schema> | undefined } | undefined
	// Resolves undefined once we stop waiting on a blocked upgrade; never rejects.
	let gaveUp: Promise<undefined> | undefined
	let blockedTooLong = false

	function reset() {
		openPromise = undefined
		openName = undefined
		settled = undefined
		gaveUp = undefined
		blockedTooLong = false
	}

	function closeCurrent() {
		const prev = openPromise
		if (prev) void prev.then((db) => db?.close()).catch(() => {})
		reset()
	}

	// Drop the cached handle without closing it: these callers have already lost their
	// connection, so there is nothing left to close.
	function forget(name: string) {
		if (openName !== name) return
		reset()
	}

	async function open(name: string, onBlockedTooLong: () => void) {
		let graceTimer: ReturnType<typeof setTimeout> | undefined
		try {
			let handle: IDBPDatabase<Schema> | undefined
			const db = await openDB<Schema>(name, opts.version, {
				upgrade(database) {
					opts.upgrade(database)
				},
				// Another tab is opening this database at a higher version. A held-open
				// connection blocks that upgrade indefinitely, and a blocked open never
				// settles — so the other tab would hang on whenReady() forever rather than
				// fail. Let go here; the next whenReady() reopens on the new schema.
				blocking() {
					handle?.close()
					forget(name)
				},
				blocked(currentVersion, blockedVersion) {
					console.warn(
						`userScopedDb(${baseName}): upgrade ${currentVersion}→${blockedVersion} waiting on another connection`
					)
					graceTimer ??= setTimeout(onBlockedTooLong, blockedGraceMs)
				},
				// The browser force-closed the connection (site data cleared, database
				// dropped from devtools). Every request on this handle would now throw, so
				// stop handing it out.
				terminated() {
					forget(name)
				}
			})
			handle = db
			if (opts.migrate && !migratedNames.has(name)) {
				migratedNames.add(name)
				try {
					await opts.migrate(db, { openDB, deleteDB })
				} catch (e) {
					// A failed migration is non-fatal: the (open) DB is still usable, so
					// we log and return it — unlike a failed open below, which yields
					// undefined. Worst case the legacy claim is missed, not the store.
					console.error(`userScopedDb(${baseName}): migration failed`, e)
				}
			}
			return db
		} catch (e) {
			// Failed open (corrupt / private-browsing): degrade to in-memory by resolving
			// undefined (callers no-op their writes).
			console.error(`userScopedDb(${baseName}): could not open database`, e)
			return undefined
		} finally {
			if (graceTimer) clearTimeout(graceTimer)
		}
	}

	function start(name: string) {
		openName = name
		let giveUp!: () => void
		gaveUp = new Promise<undefined>((resolve) => (giveUp = () => resolve(undefined)))
		const p = open(name, () => {
			blockedTooLong = true
			giveUp()
		})
		openPromise = p
		void p.then((db) => {
			// Superseded by a user switch while this was in flight: that name owns the handle.
			if (openPromise !== p) return
			settled = { db }
			blockedTooLong = false
		})
	}

	return {
		whenReady() {
			const name = scopedKey(baseName)
			if (!name) {
				closeCurrent()
				return Promise.resolve(undefined)
			}
			if (name !== openName) {
				closeCurrent()
				start(name)
			}
			if (settled) return Promise.resolve(settled.db)
			// Open requests queue per database, so a second one would wait behind this
			// still-blocked first and never fire its own `blocked` — keep the one request and
			// let callers past it instead. It stays live, so `settled` fills in the moment the
			// blocker lets go and the store comes back for the rest of the page's life.
			if (blockedTooLong) return Promise.resolve(undefined)
			return Promise.race([openPromise!, gaveUp!])
		},
		close() {
			closeCurrent()
		}
	}
}
