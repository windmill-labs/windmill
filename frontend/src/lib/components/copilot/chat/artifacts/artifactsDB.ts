// Scoped by sessionId (fixed for the session's life), not chatId: a session follows its
// active chat's rotation, so chatId-keying would drop artifacts on each new conversation.
import { type DBSchema as IDBSchema } from 'idb'
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
}

// User-scoped like the chat-history store these are keyed against: no cross-user
// co-residency on a shared browser.
const dbh = userScopedDb<ArtifactsSchema>('copilot-artifacts', {
	version: 1,
	upgrade(db) {
		const store = db.createObjectStore('items', { keyPath: 'id' })
		store.createIndex('by-session', 'sessionId')
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

export async function deleteArtifact(id: string): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		await db.delete('items', id)
	} catch (err) {
		console.error('Could not delete artifact', err)
	}
}

export async function deleteArtifactsForSession(sessionId: string): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		const tx = db.transaction('items', 'readwrite')
		const index = tx.store.index('by-session')
		let cursor = await index.openCursor(sessionId)
		while (cursor) {
			await cursor.delete()
			cursor = await cursor.continue()
		}
		await tx.done
	} catch (err) {
		console.error('Could not delete artifacts for session', err)
	}
}
