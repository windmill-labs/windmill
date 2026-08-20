// Scoped by sessionId (fixed for the session's life), not chatId: a session follows its
// active chat's rotation, so chatId-keying would drop artifacts on each new conversation.
import { type DBSchema as IDBSchema, type IDBPObjectStore, type IDBPTransaction } from 'idb'
import { userScopedDb } from '$lib/userScopedDb'

export type ArtifactKind = 'md' | 'html'

export interface PersistedArtifact {
	id: string
	sessionId: string
	chatId?: string
	kind: ArtifactKind
	/** What the artifact is for, where that outlives the session — as opposed to `kind`,
	 * which is its format. Optional, so records written before it read as undefined. */
	role?: 'plan'
	/** The version that stands as the agreed plan; below the current one means the newest
	 * text is undecided. Only exit_plan_mode can leave it behind. */
	approvedVersion?: number
	name: string
	content: string
	createdAt: number
	updatedAt: number
	/** Absent on artifacts written before history existed — read it through currentVersion(). */
	version?: number
}

/** A content snapshot taken every time an artifact's content changes. */
export interface ArtifactVersion {
	/** versionKey(artifactId, version) — the store's keyPath. */
	key: string
	artifactId: string
	version: number
	name: string
	content: string
	savedAt: number
	/** What this edit changed, in the editor's words. Absent on a first version. */
	note?: string
}

/** Oldest snapshots past this are dropped: history is bounded, IndexedDB quota is not. */
export const MAX_VERSIONS_PER_ARTIFACT = 20

/**
 * A count alone does not bound storage — twenty snapshots of a max-size artifact would be
 * ~5 MB of history for one document — so a large artifact keeps proportionally fewer.
 * Sized from the incoming snapshot rather than from the whole history, which would mean
 * deserializing every stored version's content on each write just to total it up.
 */
export const MAX_VERSION_CHARS_PER_ARTIFACT = 1024 * 1024

/** However large the artifact, keep enough history for the picker to be worth opening. */
export const MIN_VERSIONS_PER_ARTIFACT = 3

function versionsToKeep(chars: number): number {
	const affordable = Math.floor(MAX_VERSION_CHARS_PER_ARTIFACT / Math.max(1, chars))
	return Math.min(MAX_VERSIONS_PER_ARTIFACT, Math.max(MIN_VERSIONS_PER_ARTIFACT, affordable))
}

export function currentVersion(a: Pick<PersistedArtifact, 'version'>): number {
	return a.version ?? 1
}

// Re-exported so the artifact modules keep asking one module about storage; the definitions sit
// in planIdentity because that module has to stay import-free, and this one cannot.
export { isPlanArtifact, planArtifactId } from './planIdentity'

export function versionKey(artifactId: string, version: number): string {
	return `${artifactId}:${version}`
}

export function artifactFilename(a: Pick<PersistedArtifact, 'name' | 'kind'>): string {
	return `${a.name}.${a.kind === 'html' ? 'html' : 'md'}`
}

export function artifactMimeType(kind: ArtifactKind): string {
	return kind === 'html' ? 'text/html' : 'text/markdown'
}

interface ArtifactsSchema extends IDBSchema {
	items: {
		key: string
		value: PersistedArtifact
		indexes: { 'by-session': string }
	}
	versions: {
		key: string
		value: ArtifactVersion
		indexes: { 'by-artifact': string }
	}
}

// User-scoped like the chat-history store these are keyed against: no cross-user
// co-residency on a shared browser.
const dbh = userScopedDb<ArtifactsSchema>('copilot-artifacts', {
	version: 2,
	// Runs for a fresh database and for the v1 upgrade alike, so create each store only
	// when it is missing.
	upgrade(db) {
		if (!db.objectStoreNames.contains('items')) {
			const store = db.createObjectStore('items', { keyPath: 'id' })
			store.createIndex('by-session', 'sessionId')
		}
		if (!db.objectStoreNames.contains('versions')) {
			const store = db.createObjectStore('versions', { keyPath: 'key' })
			store.createIndex('by-artifact', 'artifactId')
		}
	}
})

function getDB() {
	return dbh.whenReady()
}

export async function putArtifact(artifact: PersistedArtifact): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		// A rejected write (most likely QuotaExceededError) leaves the artifact usable for the
		// session but unpersisted — degrade like the reads rather than throwing at the caller.
		await db.put('items', artifact)
	} catch (err) {
		console.error('Could not persist artifact', err)
	}
}

export async function getArtifact(id: string): Promise<PersistedArtifact | undefined> {
	const db = await getDB()
	if (!db) return undefined
	try {
		return await db.get('items', id)
	} catch (err) {
		console.error('Could not read artifact', err)
		return undefined
	}
}

export async function listArtifactsForSession(sessionId: string): Promise<PersistedArtifact[]> {
	const db = await getDB()
	if (!db) return []
	try {
		return await db.getAllFromIndex('items', 'by-session', sessionId)
	} catch (err) {
		console.error('Could not read artifacts', err)
		return []
	}
}

export interface ArtifactEdit {
	artifact: PersistedArtifact
	snapshots: ArtifactVersion[]
}

async function writeEdit(
	items: ItemsStore,
	versions: VersionsStore,
	edit: ArtifactEdit
): Promise<void> {
	await items.put(edit.artifact)
	for (const entry of edit.snapshots) await versions.put(entry)
	const newest = edit.snapshots.at(-1)
	if (newest) {
		await pruneVersionsIn(
			versions,
			edit.artifact.id,
			newest.content.length,
			edit.artifact.approvedVersion
		)
	}
}

/** `unavailable` is no database at all (private browsing), where nothing was refused —
 * distinct from `rejected`, the database turning this write down. */
export type WriteOutcome = 'saved' | 'rejected' | 'unavailable'

export interface ArtifactWrite {
	outcome: WriteOutcome
	/** The edited row, persisted or not; absent only when the mutator wrote nothing. */
	artifact?: PersistedArtifact
}

/**
 * Read an artifact and write it back in one transaction. `mutate` returns the edit to
 * write, or undefined to leave the artifact alone and resolve to no artifact. A store that
 * fails is reported rather than thrown, so the edited row resolves either way — persisted
 * where it could be, and usable for the session where it could not.
 *
 * Most callers read only `artifact` and degrade as the reads do. A caller whose write
 * carries a *constraint* reads `outcome` instead: the plan cannot be approved on the
 * strength of a row that would be gone on reload.
 */
export async function mutateArtifact(
	id: string,
	mutate: (
		existing: PersistedArtifact | undefined,
		/** The snapshot `opts.readVersion` named, when it was asked for and is still stored. */
		snapshot?: ArtifactVersion
	) => ArtifactEdit | undefined,
	opts?: { readVersion?: number }
): Promise<ArtifactWrite> {
	const db = await getDB()
	// `transaction()` throws on a connection closed since `getDB()` answered — another tab
	// upgrading the schema, or a user switch releasing the handle.
	let opened: IDBPTransaction<ArtifactsSchema, ('items' | 'versions')[], 'readwrite'> | undefined
	try {
		opened = db?.transaction(['items', 'versions'], 'readwrite')
	} catch (err) {
		console.error('Could not open an artifact write transaction', err)
	}
	// Not `return undefined`: `create` can hand out an artifact the store never took, and it
	// stays revisable only if the edit is computed anyway.
	if (!opened) return { outcome: 'unavailable', artifact: mutate(undefined)?.artifact }
	const tx = opened
	// Attached before the first await: idb builds `done` eagerly and rejects it on abort, so
	// attaching later would leave an unhandled rejection. Cleared before each deliberate
	// abort below, whose own site reports the failure when there was one.
	let reportFailure = true
	const settled: Promise<WriteOutcome> = tx.done.then(
		() => 'saved',
		(err) => {
			if (reportFailure) console.error('Could not persist artifact', err)
			return 'rejected'
		}
	)
	const abort = () => {
		try {
			tx.abort()
		} catch {}
	}
	const items = tx.objectStore('items')
	const versions = tx.objectStore('versions')
	let existing: PersistedArtifact | undefined
	let snapshot: ArtifactVersion | undefined
	try {
		// Read outside this transaction, two tabs both see version N, both stamp N+1, and the
		// later write silently replaces the earlier one — content and snapshot alike.
		existing = await items.get(id)
		// In the same transaction for the same reason: an edit conditioned on a version still
		// being readable must not weigh it against one another tab pruned in between.
		if (opts?.readVersion !== undefined) {
			snapshot = await versions.get(versionKey(id, opts.readVersion))
		}
	} catch (err) {
		console.error('Could not read the artifact being written', err)
		reportFailure = false
		abort()
		return { outcome: await settled, artifact: mutate(undefined)?.artifact }
	}
	// Kept out of the store's own error handling: a mutator that fails is not the store
	// failing, so its error is neither reported as one nor swallowed.
	let edit: ArtifactEdit | undefined
	try {
		edit = mutate(existing, snapshot)
	} catch (err) {
		reportFailure = false
		abort()
		await settled
		throw err
	}
	// Nothing to write, so nothing was refused either.
	if (!edit) {
		reportFailure = false
		abort()
		await settled
		return { outcome: 'saved' }
	}
	try {
		await writeEdit(items, versions, edit)
	} catch (err) {
		// A request that errors aborts the transaction itself; one that throws before creating
		// a request (DataCloneError) would otherwise commit the row without its snapshot.
		// Logged here because `settled` sees only a cause-less AbortError.
		console.error('Could not persist artifact', err)
		reportFailure = false
		abort()
	}
	return { outcome: await settled, artifact: edit.artifact }
}

/**
 * Drop the oldest snapshots past the budget, except the one `protect` names.
 *
 * A plan approved at v1 and planned against for twenty more rounds would otherwise lose the
 * version that stands as agreed. Excluded from the candidates rather than added on top, so
 * the budget is unchanged and what survives simply stops being contiguous.
 */
async function pruneVersionsIn(
	store: VersionsStore,
	artifactId: string,
	newestChars: number,
	protect?: number
): Promise<void> {
	const keys = await store.index('by-artifact').getAllKeys(artifactId)
	const keep = versionsToKeep(newestChars)
	if (keys.length <= keep) return
	const protectedKey = protect === undefined ? undefined : versionKey(artifactId, protect)
	// Keys sort lexicographically, which puts ":10" before ":2" — order by the parsed
	// number so pruning drops the genuinely oldest snapshots.
	const oldest = keys
		.sort((a, b) => versionOf(a) - versionOf(b))
		.filter((key) => key !== protectedKey)
		.slice(0, keys.length - keep)
	for (const key of oldest) await store.delete(key)
}

function versionOf(key: string): number {
	return Number(key.slice(key.lastIndexOf(':') + 1))
}

/** The artifact's snapshots, newest first. */
export async function listArtifactVersions(artifactId: string): Promise<ArtifactVersion[]> {
	const db = await getDB()
	if (!db) return []
	try {
		const items = await db.getAllFromIndex('versions', 'by-artifact', artifactId)
		return items.sort((a, b) => b.version - a.version)
	} catch (err) {
		console.error('Could not read artifact versions', err)
		return []
	}
}

/** A stored snapshot, or undefined when there is none. Rejects when the read could not be made
 * at all — unlike the other reads here, which degrade to undefined. A caller that conflates the
 * two reports a transient failure as permanent absence, and whatever it discards in response
 * (a reader's pinned version) is discarded for good. */
export async function getArtifactVersion(
	artifactId: string,
	version: number
): Promise<ArtifactVersion | undefined> {
	const db = await getDB()
	if (!db) throw new Error('Artifact store unavailable')
	try {
		return await db.get('versions', versionKey(artifactId, version))
	} catch (err) {
		console.error('Could not read artifact version', err)
		throw err
	}
}

export async function deleteArtifact(id: string): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		await tx.objectStore('items').delete(id)
		await deleteVersionsIn(tx.objectStore('versions'), id)
		await tx.done
	} catch (err) {
		console.error('Could not delete artifact', err)
	}
}

export async function deleteArtifactsForSession(sessionId: string): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		const items = tx.objectStore('items')
		const versions = tx.objectStore('versions')
		// Collect the ids up front rather than deleting from a live cursor: interleaving
		// another store's requests between continue() calls is what breaks a cursor walk.
		const ids = await items.index('by-session').getAllKeys(sessionId)
		for (const id of ids) {
			await items.delete(id)
			await deleteVersionsIn(versions, id)
		}
		await tx.done
	} catch (err) {
		console.error('Could not delete artifacts for session', err)
	}
}

type ItemsStore = IDBPObjectStore<ArtifactsSchema, ('items' | 'versions')[], 'items', 'readwrite'>

type VersionsStore = IDBPObjectStore<
	ArtifactsSchema,
	('items' | 'versions')[],
	'versions',
	'readwrite'
>

async function deleteVersionsIn(store: VersionsStore, artifactId: string): Promise<void> {
	for (const key of await store.index('by-artifact').getAllKeys(artifactId)) {
		await store.delete(key)
	}
}
