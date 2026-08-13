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

/**
 * Write an artifact and the snapshots that edit produced in one transaction.
 *
 * Never as two writes: a row stamped version N whose snapshot is missing still *reads*
 * as complete, because listVersions synthesizes N from the row itself — until the next
 * edit overwrites that row, at which point N's content is gone and the history has a
 * hole nothing can back-fill.
 */
export async function putArtifactWithVersions(
	artifact: PersistedArtifact,
	snapshots: ArtifactVersion[]
): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		await writeEdit(tx.objectStore('items'), tx.objectStore('versions'), { artifact, snapshots })
		await tx.done
	} catch (err) {
		// A rejected write (most likely QuotaExceededError) leaves the artifact usable for the
		// session but unpersisted — degrade like the reads rather than throwing at the caller.
		console.error('Could not persist artifact', err)
	}
}

async function writeEdit(
	items: ItemsStore,
	versions: VersionsStore,
	edit: ArtifactEdit
): Promise<void> {
	await items.put(edit.artifact)
	for (const entry of edit.snapshots) await versions.put(entry)
	const newest = edit.snapshots.at(-1)
	if (newest) await pruneVersionsIn(versions, edit.artifact.id, newest.content.length)
}

/**
 * Read an artifact and write it back in one transaction. `mutate` returns the edit to
 * write, or undefined to leave the artifact alone and resolve to undefined. A store that
 * fails is reported rather than thrown, so the edited row resolves either way — persisted
 * where it could be, and usable for the session where it could not.
 */
export async function mutateArtifact(
	id: string,
	mutate: (existing: PersistedArtifact | undefined) => ArtifactEdit | undefined
): Promise<PersistedArtifact | undefined> {
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
	if (!opened) return mutate(undefined)?.artifact
	const tx = opened
	// Attached before the first await: idb builds `done` eagerly and rejects it on abort, so
	// attaching later would leave an unhandled rejection. Cleared before each deliberate
	// abort below, whose own site reports the failure when there was one.
	let reportFailure = true
	const settled = tx.done.catch((err) => {
		if (reportFailure) console.error('Could not persist artifact', err)
	})
	const abort = () => {
		try {
			tx.abort()
		} catch {}
	}
	const items = tx.objectStore('items')
	const versions = tx.objectStore('versions')
	let existing: PersistedArtifact | undefined
	try {
		// Read outside this transaction, two tabs both see version N, both stamp N+1, and the
		// later write silently replaces the earlier one — content and snapshot alike.
		existing = await items.get(id)
	} catch (err) {
		console.error('Could not read the artifact being written', err)
		reportFailure = false
		abort()
		await settled
		return mutate(undefined)?.artifact
	}
	// Kept out of the store's own error handling: a mutator that fails is not the store
	// failing, so its error is neither reported as one nor swallowed.
	let edit: ArtifactEdit | undefined
	try {
		edit = mutate(existing)
	} catch (err) {
		reportFailure = false
		abort()
		await settled
		throw err
	}
	if (!edit) {
		reportFailure = false
		abort()
		await settled
		return undefined
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
	await settled
	return edit.artifact
}

async function pruneVersionsIn(
	store: VersionsStore,
	artifactId: string,
	newestChars: number
): Promise<void> {
	const keys = await store.index('by-artifact').getAllKeys(artifactId)
	const keep = versionsToKeep(newestChars)
	if (keys.length <= keep) return
	// Keys sort lexicographically, which puts ":10" before ":2" — order by the parsed
	// number so pruning drops the genuinely oldest snapshots.
	const oldest = keys.sort((a, b) => versionOf(a) - versionOf(b)).slice(0, keys.length - keep)
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
