/**
 * IndexedDB persistence for AI-chat linked files, keyed by session id.
 *
 * Two kinds of records survive a reload (see the persistence plan):
 *  - handle records ('file-handle' / 'dir-handle'): a re-grantable File System
 *    Access handle is stored (structured-clone), re-read live on restore.
 *  - 'snapshot' records: a full-byte Blob copy (fallback when the File System
 *    Access API is unavailable).
 *
 * Mirrors the `idb` usage in HistoryManager.svelte.ts.
 */
import { openDB, type DBSchema as IDBSchema, type IDBPDatabase } from 'idb'

export type AttachedItemKind = 'snapshot' | 'dir-handle'

export interface PersistedAttachedItem {
	/** Stable record id. */
	id: string
	sessionId: string
	/** 'snapshot' = a file copied into IndexedDB; 'dir-handle' = a live folder handle. */
	kind: AttachedItemKind
	/** Display name: relative path for files, folder name for dir-handle records. */
	name: string
	/** Raw pre-sanitization source name — the re-link identity. Absent on records
	 * persisted before provenance existed (dedupe then falls back to `name`). */
	sourceName?: string
	/** Top-level folder (for grouping); equals `name` for dir-handle records. */
	folder?: string
	/** Folder-relative path (snapshot folder children) — restores the folder grouping/tree. */
	relPath?: string
	/** Live directory handle (for 'dir-handle'). */
	handle?: FileSystemDirectoryHandle
	/** Full-content copy (for 'snapshot'). */
	blob?: Blob
	size?: number
	lastModified?: number
	addedAt: number
}

interface AttachedFilesSchema extends IDBSchema {
	items: {
		key: string
		value: PersistedAttachedItem
		indexes: { 'by-session': string }
	}
}

let dbPromise: Promise<IDBPDatabase<AttachedFilesSchema> | undefined> | undefined

function getDB(): Promise<IDBPDatabase<AttachedFilesSchema> | undefined> {
	if (!dbPromise) {
		try {
			dbPromise = openDB<AttachedFilesSchema>('copilot-attached-files', 1, {
				upgrade(db) {
					if (!db.objectStoreNames.contains('items')) {
						const store = db.createObjectStore('items', { keyPath: 'id' })
						store.createIndex('by-session', 'sessionId')
					}
				}
			}).catch((err) => {
				console.error('Could not open attached-files database', err)
				return undefined
			})
		} catch (err) {
			// IndexedDB unavailable (e.g. private mode / no DOM) — degrade gracefully.
			console.error('Could not open attached-files database', err)
			dbPromise = Promise.resolve(undefined)
		}
	}
	return dbPromise
}

export async function putItem(item: PersistedAttachedItem): Promise<void> {
	const db = await getDB()
	await db?.put('items', item)
}

export async function getItemsForSession(sessionId: string): Promise<PersistedAttachedItem[]> {
	const db = await getDB()
	if (!db) return []
	try {
		return await db.getAllFromIndex('items', 'by-session', sessionId)
	} catch (err) {
		console.error('Could not read attached files', err)
		return []
	}
}

export async function deleteItem(id: string): Promise<void> {
	const db = await getDB()
	await db?.delete('items', id)
}

export async function deleteItemsForSession(sessionId: string): Promise<void> {
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
		console.error('Could not delete attached files for session', err)
	}
}

/** Ask the browser to keep our storage from being evicted (best-effort, once). */
let persistRequested = false
export async function ensurePersistentStorage(): Promise<void> {
	if (persistRequested) return
	persistRequested = true
	try {
		await navigator.storage?.persist?.()
	} catch {
		// best-effort; ignore
	}
}
