// Scoped by sessionId (fixed for the session's life), not chatId: a session follows its
// active chat's rotation, so chatId-keying would drop the plan mid-run.
import { type DBSchema as IDBSchema } from 'idb'
import { userScopedDb } from '$lib/userScopedDb'

export type TaskStatus = 'pending' | 'in_progress' | 'completed'

export interface PersistedTask {
	sessionId: string
	// 1-based per session. This is also the id the model handles: it echoes an id on
	// every update_task call, so an integer costs a character where a UUID costs 36.
	seq: number
	subject: string
	description: string
	/** Present continuous ("Fixing auth redirect"), shown while the task is in_progress. */
	activeForm?: string
	status: TaskStatus
	createdAt: number
	updatedAt: number
}

interface TasksSchema extends IDBSchema {
	items: {
		// Compound so the store key stays globally unique while the model-facing id
		// remains the bare `seq`.
		key: [string, number]
		value: PersistedTask
		indexes: { 'by-session': string }
	}
}

// User-scoped like the chat-history store these are keyed against: no cross-user
// co-residency on a shared browser.
const dbh = userScopedDb<TasksSchema>('copilot-tasks', {
	version: 1,
	upgrade(db) {
		const store = db.createObjectStore('items', { keyPath: ['sessionId', 'seq'] })
		store.createIndex('by-session', 'sessionId')
	}
})

function getDB() {
	return dbh.whenReady()
}

/** A task before it has been assigned its session-scoped sequence number. */
export type NewTask = Omit<PersistedTask, 'sessionId' | 'seq'>

/**
 * Append tasks, numbering them from the highest `seq` present when the write runs.
 *
 * The allocation happens INSIDE the readwrite transaction on purpose. Deriving the
 * next seq from an in-memory snapshot instead lets two tabs on the same session pick
 * the same number, and since `seq` is half the primary key, the second `put` silently
 * replaces the first tab's task rather than failing. IndexedDB serialises overlapping
 * readwrite transactions on a store, so reading the max and writing in one transaction
 * closes that window.
 *
 * Returns undefined when IndexedDB is unavailable or the write failed — the caller
 * then numbers in memory, where there is no other tab to contend with.
 */
export async function appendTasks(
	sessionId: string,
	drafts: NewTask[]
): Promise<PersistedTask[] | undefined> {
	const db = await getDB()
	if (!db) return undefined
	try {
		const tx = db.transaction('items', 'readwrite')
		let next = 0
		let cursor = await tx.store.index('by-session').openCursor(IDBKeyRange.only(sessionId))
		while (cursor) {
			next = Math.max(next, cursor.value.seq)
			cursor = await cursor.continue()
		}
		const created = drafts.map((draft) => ({ ...draft, sessionId, seq: ++next }))
		for (const task of created) await tx.store.put(task)
		await tx.done
		return created
	} catch (err) {
		console.error('Could not persist tasks', err)
		return undefined
	}
}

export async function putTask(task: PersistedTask): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		// A rejected write (most likely QuotaExceededError) leaves the task usable for the
		// session but unpersisted — degrade like the reads rather than throwing at the caller.
		await db.put('items', task)
	} catch (err) {
		console.error('Could not persist task', err)
	}
}

export async function listTasksForSession(sessionId: string): Promise<PersistedTask[]> {
	const db = await getDB()
	if (!db) return []
	try {
		return await db.getAllFromIndex('items', 'by-session', sessionId)
	} catch (err) {
		console.error('Could not read tasks', err)
		return []
	}
}

export async function deleteTask(sessionId: string, seq: number): Promise<void> {
	const db = await getDB()
	if (!db) return
	try {
		await db.delete('items', [sessionId, seq])
	} catch (err) {
		console.error('Could not delete task', err)
	}
}

export async function deleteTasksForSession(sessionId: string): Promise<void> {
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
		console.error('Could not delete tasks for session', err)
	}
}
