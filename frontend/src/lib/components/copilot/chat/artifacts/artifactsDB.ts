// Scoped by sessionId (fixed for the session's life), not chatId: a session follows its
// active chat's rotation, so chatId-keying would drop artifacts on each new conversation.
import { type DBSchema as IDBSchema, type IDBPObjectStore, type IDBPTransaction } from 'idb'
import { userScopedDb } from '$lib/userScopedDb'
import { emailOfScopedKey, scopedKeyFor } from '$lib/userScopedStorage'
import { markSessionDirty } from '$lib/components/sessions/sessionMirrorSignal'

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

const ARTIFACTS_DB = 'copilot-artifacts'

// User-scoped like the chat-history store these are keyed against: no cross-user
// co-residency on a shared browser.
const dbh = userScopedDb<ArtifactsSchema>(ARTIFACTS_DB, {
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
		markSessionDirty(artifact.sessionId, undefined, emailOfScopedKey(ARTIFACTS_DB, db.name))
	} catch (err) {
		console.error('Could not persist artifact', err)
	}
}

/** A session's artifacts with their history, or undefined when the store is unavailable
 * or no longer the named user's (see `readStoredSessions`). */
export async function readSessionArtifacts(
	sessionId: string,
	email: string
): Promise<{ items: PersistedArtifact[]; versions: ArtifactVersion[] } | undefined> {
	const db = await getDB()
	if (!db || db.name !== scopedKeyFor(ARTIFACTS_DB, email)) return undefined
	try {
		const items = await db.getAllFromIndex('items', 'by-session', sessionId)
		const versions = (
			await Promise.all(items.map((i) => db.getAllFromIndex('versions', 'by-artifact', i.id)))
		).flat()
		return { items, versions }
	} catch (err) {
		console.error('Could not read artifacts', err)
		return undefined
	}
}

/** Write restored artifacts and snapshots, leaving any that already exist alone. Reports
 * whether they are all in the store now: unlike the other writes here, a caller records the
 * restore as done on the strength of this answer. */
export async function importArtifacts(
	items: PersistedArtifact[],
	versions: ArtifactVersion[],
	email: string,
	overwrite = false
): Promise<boolean> {
	if (items.length === 0 && versions.length === 0) return true
	const db = await getDB()
	if (!db || db.name !== scopedKeyFor(ARTIFACTS_DB, email)) return false
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		const itemStore = tx.objectStore('items')
		const versionStore = tx.objectStore('versions')
		// An overwrite never puts an older record over a newer one: without a cross-tab lock,
		// another restore may have landed a newer backup's copy meanwhile.
		for (const item of items) {
			const existing = await itemStore.get(item.id)
			if (existing === undefined || (overwrite && existing.updatedAt <= item.updatedAt)) {
				await itemStore.put(item)
			}
		}
		for (const version of versions) {
			if (overwrite || (await versionStore.getKey(version.key)) === undefined) {
				await versionStore.put(version)
			}
		}
		await tx.done
		return true
	} catch (err) {
		console.error('Could not import artifacts', err)
		return false
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

/** What a mutator decided: the edit to write, or why it wrote none. */
export type Mutation<R> = { edit: ArtifactEdit } | { refused: R }

export interface ArtifactWrite<R = never> {
	/** `saved` too when the mutator wrote nothing. */
	outcome: WriteOutcome
	/** The row the edit wrote; absent when there was none, and when the store refused it. */
	artifact?: PersistedArtifact
	/** The mutator's answer when it wrote no edit. */
	refused?: R
}

/**
 * Read an artifact and write it back in one transaction. `mutate` returns the edit to write, a
 * refusal to hand back, or undefined to leave the artifact alone. A store that fails is reported
 * in `outcome` rather than thrown, and the mutator is not asked when there is no store to read.
 */
export async function mutateArtifact<R = never>(
	id: string,
	mutate: Mutator<R>,
	opts?: { readVersion?: number }
): Promise<ArtifactWrite<R>> {
	// Asked again on the retry, so the version check runs against what the store holds then.
	const { full, ...first } = await mutateOnce(id, mutate, opts)
	if (!full || !(await makeRoom())) return first
	const { full: _, ...retried } = await mutateOnce(id, mutate, opts)
	return retried
}

type Mutator<R> = (
	existing: PersistedArtifact | undefined,
	/** The snapshot `opts.readVersion` named, when it was asked for and is still stored. */
	snapshot?: ArtifactVersion
) => Mutation<R> | undefined

/** Whether the browser refused a write for want of space. */
export function isQuotaError(err: unknown): boolean {
	return (err as { name?: unknown } | null | undefined)?.name === 'QuotaExceededError'
}

/**
 * Free space after the browser refused a write for want of it: cut each artifact's history to
 * its latest MIN_VERSIONS_PER_ARTIFACT snapshots, keeping a plan's approved one. Resolves to
 * whether anything was deleted, so only a write that might now fit is tried again.
 */
export async function makeRoom(): Promise<boolean> {
	const db = await getDB()
	if (!db) return false
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		const items = tx.objectStore('items')
		const versions = tx.objectStore('versions')
		const touched = new Set<string>()
		for (const a of await items.getAll()) {
			if ((await dropOldestIn(versions, a.id, MIN_VERSIONS_PER_ARTIFACT, a.approvedVersion)) > 0) {
				touched.add(a.sessionId)
			}
		}
		await tx.done
		const email = emailOfScopedKey(ARTIFACTS_DB, db.name)
		for (const sessionId of touched) markSessionDirty(sessionId, undefined, email)
		return touched.size > 0
	} catch (err) {
		console.error('Could not free space for artifacts', err)
		return false
	}
}

async function mutateOnce<R>(
	id: string,
	mutate: Mutator<R>,
	opts?: { readVersion?: number }
): Promise<ArtifactWrite<R> & { full?: boolean }> {
	let full = false
	const db = await getDB()
	// `transaction()` throws on a connection closed since `getDB()` answered — another tab
	// upgrading the schema, or a user switch releasing the handle.
	let opened: IDBPTransaction<ArtifactsSchema, ('items' | 'versions')[], 'readwrite'> | undefined
	try {
		opened = db?.transaction(['items', 'versions'], 'readwrite')
	} catch (err) {
		console.error('Could not open an artifact write transaction', err)
	}
	if (!opened) return { outcome: 'unavailable' }
	const tx = opened
	// Attached before the first await: idb builds `done` eagerly and rejects it on abort, so
	// attaching later would leave an unhandled rejection. Cleared before each deliberate
	// abort below, whose own site reports the failure when there was one.
	let reportFailure = true
	const settled: Promise<WriteOutcome> = tx.done.then(
		() => 'saved',
		(err) => {
			if (reportFailure) console.error('Could not persist artifact', err)
			full ||= isQuotaError(err)
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
		return { outcome: await settled }
	}
	// Kept out of the store's own error handling: a mutator that fails is not the store
	// failing, so its error is neither reported as one nor swallowed.
	let decided: Mutation<R> | undefined
	try {
		decided = mutate(existing, snapshot)
	} catch (err) {
		reportFailure = false
		abort()
		await settled
		throw err
	}
	// Nothing to write, so nothing was refused either.
	if (!decided || !('edit' in decided)) {
		reportFailure = false
		abort()
		await settled
		return { outcome: 'saved', refused: decided?.refused }
	}
	const edit = decided.edit
	try {
		await writeEdit(items, versions, edit)
	} catch (err) {
		// A request that errors aborts the transaction itself; one that throws before creating
		// a request (DataCloneError) would otherwise commit the row without its snapshot.
		// Logged here because `settled` sees only a cause-less AbortError.
		console.error('Could not persist artifact', err)
		full = isQuotaError(err)
		reportFailure = false
		abort()
	}
	const outcome = await settled
	if (outcome !== 'saved') return { outcome, full }
	if (db) {
		markSessionDirty(edit.artifact.sessionId, undefined, emailOfScopedKey(ARTIFACTS_DB, db.name))
	}
	return { outcome, artifact: edit.artifact }
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
	await dropOldestIn(store, artifactId, versionsToKeep(newestChars), protect)
}

/** Delete all but the `keep` newest snapshots, never the one `protect` names. Resolves to how
 * many were deleted. */
async function dropOldestIn(
	store: VersionsStore,
	artifactId: string,
	keep: number,
	protect?: number
): Promise<number> {
	const keys = await store.index('by-artifact').getAllKeys(artifactId)
	if (keys.length <= keep) return 0
	const protectedKey = protect === undefined ? undefined : versionKey(artifactId, protect)
	// Keys sort lexicographically, which puts ":10" before ":2" — order by the parsed
	// number so pruning drops the genuinely oldest snapshots.
	const oldest = keys
		.sort((a, b) => versionOf(a) - versionOf(b))
		.filter((key) => key !== protectedKey)
		.slice(0, keys.length - keep)
	for (const key of oldest) await store.delete(key)
	return oldest.length
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
		const items = tx.objectStore('items')
		const sessionId = (await items.get(id))?.sessionId
		await items.delete(id)
		await deleteVersionsIn(tx.objectStore('versions'), id)
		await tx.done
		if (sessionId) markSessionDirty(sessionId, undefined, emailOfScopedKey(ARTIFACTS_DB, db.name))
	} catch (err) {
		console.error('Could not delete artifact', err)
	}
}

/** Deletes these artifacts of the session (with their versions) and these versions: what
 * an earlier restore staged for it and the backup no longer has. False when nothing could
 * be deleted. */
export async function pruneSessionArtifacts(
	sessionId: string,
	itemIds: Set<string>,
	versionKeys: Set<string>,
	email: string
): Promise<boolean> {
	if (itemIds.size === 0 && versionKeys.size === 0) return true
	const db = await getDB()
	if (!db || db.name !== scopedKeyFor(ARTIFACTS_DB, email)) return false
	try {
		const tx = db.transaction(['items', 'versions'], 'readwrite')
		const items = tx.objectStore('items')
		const versions = tx.objectStore('versions')
		for (const id of await items.index('by-session').getAllKeys(sessionId)) {
			if (!itemIds.has(String(id))) continue
			await items.delete(id)
			await deleteVersionsIn(versions, String(id))
		}
		for (const key of versionKeys) await versions.delete(key)
		await tx.done
		return true
	} catch (err) {
		console.error('Could not prune artifacts for session', err)
		return false
	}
}

/** False when the store could not be reached or the deletion failed. With `email`, only that
 * user's store is touched: a caller that captured its user must not follow an account switch. */
export async function deleteArtifactsForSession(
	sessionId: string,
	email?: string
): Promise<boolean> {
	const db = await getDB()
	if (!db || (email !== undefined && db.name !== scopedKeyFor(ARTIFACTS_DB, email))) return false
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
		return true
	} catch (err) {
		console.error('Could not delete artifacts for session', err)
		return false
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
