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
// session runtime) — a per-instance subscription would leak callbacks. The email
// can change back and forth within one page: logout is a client-side goto() that
// clears userStore, not a reload.

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
	// How long an open waits to settle before giving up. Injectable for tests, which
	// cannot afford the real grace period.
	openGraceMs?: number
}

export interface UserScopedDb<Schema extends DBSchema> {
	// Resolves to the open DB for the current user, or undefined when no user is logged
	// in yet, the open failed, or another connection is holding up a schema upgrade —
	// directly, or by sitting ahead of this open in the browser's queue for the database
	// (degrade to in-memory; never rejects, never hangs).
	whenReady(): Promise<IDBPDatabase<Schema> | undefined>
	close(): void
}

// How long an open waits to settle before the opener gives up and degrades to in-memory.
// Only a connection that ignores `versionchange`, or an open queued behind one, takes this long.
const OPEN_GRACE_MS = 5000

export function userScopedDb<Schema extends DBSchema>(
	baseName: string,
	opts: UserScopedDbOptions<Schema>
): UserScopedDb<Schema> {
	const openDB = opts.openDB ?? idbOpenDB
	const deleteDB = opts.deleteDB ?? idbDeleteDB
	const openGraceMs = opts.openGraceMs ?? OPEN_GRACE_MS
	const migratedNames = new Set<string>()

	/**
	 * One open request for one scoped database, plus everything learned about it since.
	 *
	 * An open cannot be cancelled and the browser processes a database's opens in order, so
	 * a replacement issued while one is pending waits behind it, never reaching its own
	 * `blocked` callback. An attempt is therefore replaced only once it has settled; losing
	 * interest in it is recorded on these fields instead.
	 */
	interface Attempt {
		/**
		 * The open with the keep-or-discard decision already applied, so a handle we drop is
		 * reported as undefined rather than handed over and closed behind the caller's back.
		 */
		outcome: Promise<IDBPDatabase<Schema> | undefined>
		/** Set when `outcome` settles. Its presence, not its `db`, is what "settled" means. */
		settled?: { db: IDBPDatabase<Schema> | undefined }
		/** Resolves undefined when the open has taken too long to settle. */
		gaveUp: Promise<undefined>
		timedOut: boolean
		/** Handle yielded to another tab's upgrade, or force-closed by the browser. */
		dead: boolean
		/** Released before it arrived: close it on arrival rather than hand it out. */
		unwanted: boolean
	}

	const attempts = new Map<string, Attempt>()
	let currentName: string | undefined

	async function open(
		name: string,
		attempt: Attempt,
		onTooLong: () => void
	): Promise<IDBPDatabase<Schema> | undefined> {
		// Armed before the request is issued rather than from `blocked`, because an open
		// queued behind another one is told nothing at all: it waits its turn in the
		// browser's per-database queue, and a callback that never fires cannot bound it.
		let graceTimer: ReturnType<typeof setTimeout> | undefined = setTimeout(onTooLong, openGraceMs)
		const stopWaiting = () => {
			if (graceTimer) clearTimeout(graceTimer)
			graceTimer = undefined
		}
		try {
			let handle: IDBPDatabase<Schema> | undefined
			const db = await openDB<Schema>(name, opts.version, {
				upgrade(database) {
					// The version-change transaction is ours: nothing is queued ahead of this
					// open any more, and what remains is our own upgrade running.
					stopWaiting()
					opts.upgrade(database)
				},
				// Another tab is opening this database at a higher version, which our open
				// connection would block indefinitely. Let go so their upgrade lands; this
				// attempt is replaced by a fresh open once it has settled.
				blocking() {
					handle?.close()
					attempt.dead = true
				},
				blocked(currentVersion, blockedVersion) {
					console.warn(
						`userScopedDb(${baseName}): upgrade ${currentVersion}→${blockedVersion} waiting on another connection`
					)
				},
				// The browser force-closed the connection (site data cleared, database
				// dropped from devtools). Every request on this handle would now throw.
				terminated() {
					attempt.dead = true
				}
			})
			handle = db
			// The handle is in hand; a slow migrate is not a hung open.
			stopWaiting()
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
			// Failed open (corrupt / private-browsing / a newer version already stored):
			// degrade to in-memory by resolving undefined (callers no-op their writes).
			console.error(`userScopedDb(${baseName}): could not open database`, e)
			return undefined
		} finally {
			stopWaiting()
		}
	}

	function start(name: string): Attempt {
		let giveUp!: () => void
		const attempt: Attempt = {
			// Assigned immediately below; open() needs the attempt to report back onto.
			outcome: undefined as unknown as Promise<IDBPDatabase<Schema> | undefined>,
			gaveUp: new Promise<undefined>((resolve) => (giveUp = () => resolve(undefined))),
			timedOut: false,
			dead: false,
			unwanted: false
		}
		// The timer is owned by this attempt, so it can only ever time out its own request.
		attempt.outcome = open(name, attempt, () => {
			attempt.timedOut = true
			giveUp()
		}).then((db) => {
			// Arrived after we stopped wanting it, or after we yielded the handle: holding it
			// open would block the next tab's upgrade for no one's benefit, and by now it may
			// belong to a user who is no longer logged in.
			const discard = attempt.unwanted || attempt.dead
			if (discard) {
				db?.close()
				attempt.dead = true
			}
			attempt.settled = { db: discard ? undefined : db }
			attempt.timedOut = false
			return attempt.settled.db
		})
		attempts.set(name, attempt)
		return attempt
	}

	/** Stop serving `name`; a still-pending attempt is only marked, per the rule on Attempt. */
	function release(name: string | undefined) {
		const attempt = name ? attempts.get(name) : undefined
		if (!attempt) return
		if (attempt.settled) {
			const { db } = attempt.settled
			attempts.delete(name!)
			// A microtask later, never synchronously: callers hold this handle across several
			// awaits (hydrate, write-behind), and closing under them turns a routine user
			// switch into InvalidStateError. close() then waits on what they started.
			if (db) void Promise.resolve().then(() => db.close())
			return
		}
		attempt.unwanted = true
	}

	return {
		whenReady() {
			const name = scopedKey(baseName)
			if (name !== currentName) {
				release(currentName)
				currentName = name
			}
			if (!name) return Promise.resolve(undefined)

			let attempt = attempts.get(name)
			// The one place an attempt is replaced, and only once settled — see Attempt.
			if (attempt?.settled && attempt.dead) {
				attempts.delete(name)
				attempt = undefined
			}
			attempt ??= start(name)
			// Wanting it again cancels a release that has not landed yet.
			attempt.unwanted = false

			if (attempt.settled) return Promise.resolve(attempt.settled.db)
			// Still in flight. `dead` means the handle is spoken for and `timedOut` that we
			// stopped waiting — either way, degrade now rather than block the caller; both
			// resolve themselves when the request finally settles.
			if (attempt.dead || attempt.timedOut) return Promise.resolve(undefined)
			return Promise.race([attempt.outcome, attempt.gaveUp])
		},
		close() {
			release(currentName)
			currentName = undefined
		}
	}
}
