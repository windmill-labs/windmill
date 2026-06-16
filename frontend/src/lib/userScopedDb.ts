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
}

export interface UserScopedDb<Schema extends DBSchema> {
	// Resolves to the open DB for the current user, or undefined when no user is
	// logged in yet or the open failed (degrade to in-memory; never rejects).
	whenReady(): Promise<IDBPDatabase<Schema> | undefined>
	close(): void
}

export function userScopedDb<Schema extends DBSchema>(
	baseName: string,
	opts: UserScopedDbOptions<Schema>
): UserScopedDb<Schema> {
	const openDB = opts.openDB ?? idbOpenDB
	const deleteDB = opts.deleteDB ?? idbDeleteDB
	const migratedNames = new Set<string>()

	let openName: string | undefined
	let openPromise: Promise<IDBPDatabase<Schema> | undefined> | undefined

	function closeCurrent() {
		const prev = openPromise
		if (prev) void prev.then((db) => db?.close()).catch(() => {})
		openPromise = undefined
		openName = undefined
	}

	async function open(name: string): Promise<IDBPDatabase<Schema> | undefined> {
		try {
			const db = await openDB<Schema>(name, opts.version, {
				upgrade(database) {
					opts.upgrade(database)
				}
			})
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
			// Failed open (blocked / corrupt / private-browsing): degrade to
			// in-memory by resolving undefined (callers no-op their writes). The
			// undefined is cached for this name so we don't hammer the open.
			console.error(`userScopedDb(${baseName}): could not open database`, e)
			return undefined
		}
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
				openName = name
				openPromise = open(name)
			}
			return openPromise!
		},
		close() {
			closeCurrent()
		}
	}
}
