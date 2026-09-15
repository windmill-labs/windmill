// Lazily backs the browser's AI sessions up to their workspace's object storage, and
// restores the ones this browser does not have.
//
// IndexedDB stays the store every write lands in; the funnels there only mark a session
// dirty (sessionMirrorSignal). A flush runs once the marks have been quiet for a while,
// bounded by a maximum delay so a long turn still gets backed up part-way, and sends
// batched requests per workspace carrying only the pieces whose marker moved
// (sessionMirrorPlan). Marks are persisted, so a crash leaves them for the next load.
//
// What a flush is for is decided per workspace: `enabled: false` (no storage, or the
// admin switch) turns it off for the page.
import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { openDB, type DBSchema, type IDBPDatabase } from 'idb'
import {
	AiService,
	ApiError,
	type AISessionBackup,
	type AISessionBackupCursor,
	type AISessionBackupImage,
	type AISessionBackupPush
} from '$lib/gen'
import { userWorkspaces } from '$lib/stores'
import { userScopedDb } from '$lib/userScopedDb'
import { getCurrentUserEmail, onUserChange, scopedKey, scopedKeyFor } from '$lib/userScopedStorage'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import { workspaceRootId } from './sessionScope.svelte'
import { onMirrorSignal } from './sessionMirrorSignal'
import {
	importSessions,
	isSessionTombstoned,
	readStoredSessions,
	sessionState,
	type Session
} from './sessionState.svelte'
import {
	importStoredChats,
	pruneSessionChats,
	listChatImageIds,
	listSessionChatIds,
	readImageDataUrl,
	readStoredChat,
	type RestoredImage,
	type StoredChat
} from '../copilot/chat/HistoryManager.svelte'
import {
	importArtifacts,
	pruneSessionArtifacts,
	readSessionArtifacts,
	type ArtifactVersion,
	type PersistedArtifact
} from '../copilot/chat/artifacts/artifactsDB'
import {
	artifactsFingerprint,
	headSig,
	jsonBytes,
	operationsOf,
	planSessionPush,
	splitEntry,
	type ChatSnapshot,
	type MirrorSyncState,
	type PlannedPush,
	type PushBody
} from './sessionMirrorPlan'

/** A flush waits for the marks to go quiet this long. */
const QUIET_MS = 15_000
/** ...but never longer than this after the first unflushed mark. */
const MAX_DELAY_MS = 120_000
const STARTUP_DELAY_MS = 10_000
const RETRY_MIN_MS = 30_000
const RETRY_MAX_MS = 600_000
/** Requests are packed up to about this many bytes; the server accepts four times that. */
const REQUEST_TARGET_BYTES = 8 * 1024 * 1024
/** The server's caps per request and per entry. */
const MAX_ENTRIES_PER_REQUEST = 100
const MAX_REMOVED_PER_REQUEST = 200
const MAX_IMAGES_PER_ENTRY = 500
const MAX_OPERATIONS_PER_REQUEST = 4000
/** A chat or a session's artifacts beyond this are left out of the backup rather than
 * sent: with the record and the deletes riding along, the largest entry stays well under
 * the server's 32 MB body cap. */
const MAX_CHAT_BYTES = 16 * 1024 * 1024
const MAX_ARTIFACTS_BYTES = 8 * 1024 * 1024
const PULL_BATCH = 5
/** Newest sessions restored per workspace: every visible session gets a runtime, and each
 * runtime's history load reads the whole chat store. */
const RESTORE_MAX = 50
const PENDING_PREFIX = 'windmill_sessions_mirror_pending'
const SYNC_DB = 'windmill-sessions-mirror'

interface MirrorSchema extends DBSchema {
	sync: { key: string; value: MirrorSyncState }
}

function createSyncStore(db: IDBPDatabase<MirrorSchema>): void {
	if (!db.objectStoreNames.contains('sync')) db.createObjectStore('sync', { keyPath: 'id' })
}

const syncDbh = userScopedDb<MirrorSchema>(SYNC_DB, { version: 1, upgrade: createSyncStore })

async function syncDb(email: string) {
	const db = await syncDbh.whenReady()
	return db && db.name === scopedKeyFor(SYNC_DB, email) ? db : undefined
}

// --- Pending marks ---
//
// One localStorage key per mark, shared by every tab of the user: a dirty mark holds a
// counter bumped on every write, a removal mark the workspace to remove from. Keying each
// mark on its own is what lets two tabs mark different sessions at the same time without
// one rewriting the other's mark away, as a single JSON blob would.

interface PendingMarks {
	dirty: { id: string; v: number }[]
	removed: { id: string; ws?: string; key: string }[]
}

function pendingBase(): string | undefined {
	return scopedKey(PENDING_PREFIX)
}

/** The marks of the user a write landed for: the current user unless the signal says
 * otherwise (its store's user, when the user changed while the write was pending). */
function pendingBaseFor(email: string | undefined): string | undefined {
	return email ? scopedKeyFor(PENDING_PREFIX, email) : pendingBase()
}

function dirtyKey(base: string, sessionId: string): string {
	return `${base}::d::${sessionId}`
}

function removedKey(base: string, sessionId: string, ws: string | undefined): string {
	return `${base}::r::${sessionId}::${ws ?? ''}`
}

function readPending(): PendingMarks {
	const marks: PendingMarks = { dirty: [], removed: [] }
	const base = pendingBase()
	if (!base) return marks
	try {
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i)
			if (!key || !key.startsWith(`${base}::`)) continue
			const rest = key.slice(base.length + 2)
			if (rest.startsWith('d::')) {
				const v = Number(localStorage.getItem(key))
				marks.dirty.push({ id: rest.slice(3), v: Number.isFinite(v) ? v : 0 })
			} else if (rest.startsWith('r::')) {
				const [id, ws] = rest.slice(3).split('::')
				marks.removed.push({ id, ws: ws || undefined, key })
			}
		}
	} catch (e) {
		console.error('Could not read session backup marks', e)
	}
	// Marks this page could not write: their rows carry the bumps (or are absent), which is
	// what makes them live; the counter only says so.
	for (const id of unwritableMarks) {
		if (!marks.dirty.some((d) => d.id === id)) marks.dirty.push({ id, v: 0 })
	}
	return marks
}

/** False when the mark could not be written (storage full): the caller must carry the
 * change some other way. */
function bumpDirty(sessionId: string, email?: string): boolean {
	const base = pendingBaseFor(email)
	if (!base) return false
	try {
		const key = dirtyKey(base, sessionId)
		const v = Number(localStorage.getItem(key) ?? '0')
		localStorage.setItem(key, String((Number.isFinite(v) ? v : 0) + 1))
		return true
	} catch (e) {
		console.error('Could not persist session backup mark', e)
		return false
	}
}

/** The durable fallback for a dirty mark that could not be written: the bump goes on the
 * sync row, where a flush in flight cannot lose it (`writeSync` keeps it). A session
 * without a row is live without any mark, and marked again by every load's backfill. */
async function bumpViaSyncRow(id: string, email = getCurrentUserEmail()): Promise<void> {
	if (!email) return
	await updateSyncRow(id, email, (row) =>
		row ? { ...row, extraV: (row.extraV ?? 0) + 1 } : undefined
	)
}

/** Reads a row and writes what `update` makes of it, in the store of `email`: through the
 * shared handle when that is the current user, and through a connection of its own
 * otherwise, since the shared handle follows the current user and a write that landed
 * after a switch must still reach the store it belongs to. */
async function updateSyncRow(
	id: string,
	email: string,
	update: (row: MirrorSyncState | undefined) => MirrorSyncState | undefined
): Promise<void> {
	if (email === getCurrentUserEmail()) {
		const next = update(await readSync(id, email))
		if (next) await writeSync([next], email)
		return
	}
	let db: IDBPDatabase<MirrorSchema> | undefined
	try {
		db = await openDB<MirrorSchema>(scopedKeyFor(SYNC_DB, email), 1, { upgrade: createSyncStore })
		const next = update(await db.get('sync', id))
		if (next) await db.put('sync', next)
	} catch (e) {
		console.error('Could not update the session backup state of another user', e)
	} finally {
		db?.close()
	}
}

/** Drop the dirty mark of a session gone from the store, the one case nothing can bump
 * again. Every other mark stays: one whose push landed is retired through `flushedV` on
 * the sync row (two localStorage calls cannot compare-and-delete, and a bump landing
 * between them would be lost), and a draft's or an off workspace's waits its turn. */
function dropDirty(sessionId: string): void {
	const base = pendingBase()
	if (base) removeKey(dirtyKey(base, sessionId))
}

/** False when the mark could not be written (storage full): the caller must not act as if
 * the removal were scheduled. */
function addRemoved(
	sessionId: string,
	ws: string | undefined,
	dropDirty: boolean,
	email?: string
): boolean {
	const base = pendingBaseFor(email)
	if (!base) return false
	try {
		if (dropDirty) localStorage.removeItem(dirtyKey(base, sessionId))
		localStorage.setItem(removedKey(base, sessionId, ws), '1')
		return true
	} catch (e) {
		console.error('Could not persist session backup mark', e)
		return false
	}
}

function removeKey(key: string): void {
	try {
		localStorage.removeItem(key)
	} catch {}
}

// --- Scheduling ---

let quietTimer: ReturnType<typeof setTimeout> | undefined
let maxTimer: ReturnType<typeof setTimeout> | undefined
let retryTimer: ReturnType<typeof setTimeout> | undefined
let retryAt = 0
let retryMs = RETRY_MIN_MS
/**
 * Workspaces whose storage answered this page load. `off`: nowhere to keep backups.
 * `refused`: the server rejected what this page sends; nothing more is sent for the page,
 * but the marks and the sync state stay, so the next load tries again.
 */
const wsState = new Map<string, 'on' | 'off' | 'refused'>()
let backfilled = false
/** Sessions of the current user whose dirty mark could not be written this page. */
let unwritableMarks = new Set<string>()
const restoredWorkspaces = new Set<string>()

// One flush or restore at a time in this tab; each reads the marks fresh.
let chain: Promise<unknown> = Promise.resolve()
function enqueue<T>(fn: () => Promise<T>): Promise<T | void> {
	const run = chain.then(fn, fn)
	chain = run.catch((e) => console.error('Session backup failed', e))
	return run
}

function clearTimers(): void {
	clearTimeout(quietTimer)
	clearTimeout(maxTimer)
	quietTimer = undefined
	maxTimer = undefined
}

function scheduleFlush(): void {
	clearTimeout(quietTimer)
	quietTimer = setTimeout(runFlush, QUIET_MS)
	maxTimer ??= setTimeout(runFlush, MAX_DELAY_MS)
}

function runFlush(): void {
	clearTimers()
	void enqueue(flush)
}

function backOff(): void {
	retryAt = Date.now() + retryMs
	retryMs = Math.min(retryMs * 2, RETRY_MAX_MS)
	clearTimeout(retryTimer)
	retryTimer = setTimeout(runFlush, retryAt - Date.now())
}

/** Serialize with the other tabs of the same user where the browser lets us; on plain
 * http there is no lock, and two tabs at worst upload the same bytes twice. */
/** One tab of the user at a time in the flush and the restore. A flush finding the lock
 * taken reschedules itself; a restore waits its turn, since two tabs restoring the same
 * absent session would each write its pieces over the other's. */
async function withUserLock(email: string, fn: () => Promise<void>, wait = false): Promise<void> {
	const locks =
		typeof navigator === 'undefined' ? undefined : (navigator as { locks?: LockManager }).locks
	if (!locks) return fn()
	await locks.request(`wm-ai-sessions-mirror::${email}`, { ifAvailable: !wait }, async (lock) => {
		if (lock) await fn()
		// The other tab's flush read the marks before this one's were written: try again
		// once it is done, rather than wait for the next write or load.
		else scheduleFlush()
	})
}

// --- Flush ---

function statusOf(e: unknown): number | undefined {
	return e instanceof ApiError ? e.status : undefined
}

async function readSync(id: string, email: string): Promise<MirrorSyncState | undefined> {
	return (await syncDb(email))?.get('sync', id)
}

async function allSyncRows(email: string): Promise<MirrorSyncState[]> {
	return (await (await syncDb(email))?.getAll('sync')) ?? []
}

/** The durable fallback for a user delete whose localStorage mark could not be written.
 * A session with a row was backed up; one without may have its first push in flight, so
 * it gets a row saying only that, which the push's own row write keeps (`writeSync`). An
 * unsent draft gets nothing: it was never pushed. */
async function removeViaSyncRow(
	id: string,
	ws: string | undefined,
	email = getCurrentUserEmail()
): Promise<void> {
	if (!email) return
	await updateSyncRow(id, email, (row) =>
		row
			? { ...row, removed: true }
			: ws
				? { id, ws, head: '', chats: {}, images: {}, removed: true }
				: undefined
	)
}

/** Writes rows whole, except that a removal filed on a row meanwhile survives: the flush
 * writes a session's row from state it read before the push, and the user may have deleted
 * the session in between. */
async function writeSync(states: MirrorSyncState[], email: string): Promise<void> {
	const db = await syncDb(email)
	if (!db || states.length === 0) return
	const tx = db.transaction('sync', 'readwrite')
	for (const s of states) {
		const cur = await tx.store.get(s.id)
		const removed = s.removed || cur?.removed
		const extraV = Math.max(s.extraV ?? 0, cur?.extraV ?? 0)
		await tx.store.put({
			...s,
			...(removed ? { removed: true } : {}),
			...(extraV > 0 ? { extraV } : {})
		})
	}
	await tx.done
}

async function deleteSync(ids: string[], email: string): Promise<void> {
	const db = await syncDb(email)
	if (!db || ids.length === 0) return
	const tx = db.transaction('sync', 'readwrite')
	for (const id of ids) await tx.store.delete(id)
	await tx.done
}

/** A workspace's storage went away: what was pushed there can no longer be trusted to be
 * where a later storage looks, so every session goes whole on the next push. The rows stay
 * (stale) so a removal still knows a backup existed. */
async function staleWorkspaceSync(ws: string, email: string): Promise<void> {
	const db = await syncDb(email)
	if (!db) return
	const rows = (await db.getAll('sync')).filter((s) => s.ws === ws && !s.stale)
	await writeSync(
		rows.map((s) => ({ ...s, stale: true })),
		email
	)
}

/** The workspace's live rows recorded against another storage than the one the server
 * answers from: they describe objects it no longer looks at (a new bucket starts empty). */
function foreignRows(
	ws: string,
	storageId: string,
	generation: number,
	rows: Iterable<MirrorSyncState>
): MirrorSyncState[] {
	return [...rows].filter(
		(row) =>
			row.ws === ws &&
			!row.stale &&
			!row.removed &&
			!row.staging &&
			(row.storageId !== storageId || (row.generation ?? 0) !== generation)
	)
}

/** Stale rows plan like no row at all, and the mark makes the next flush pick them up. */
async function markStale(rows: MirrorSyncState[], email: string): Promise<void> {
	await writeSync(
		rows.map((row) => ({ ...row, stale: true })),
		email
	)
	for (const row of rows) bumpDirty(row.id)
}

/** Sessions this browser has backed up nothing of yet, or whose backup went stale:
 * everything committed to a workspace gets a mark, once per page load. */
async function backfillMarks(marks: PendingMarks, email: string): Promise<boolean> {
	if (backfilled) return true
	const sessions = await readStoredSessions(email)
	const db = await syncDb(email)
	if (!sessions || !db) return false
	backfilled = true
	const rows = new Map((await db.getAll('sync')).map((s) => [s.id, s]))
	const marked = new Set(marks.dirty.map((d) => d.id))
	for (const s of sessions) {
		const row = rows.get(s.id)
		if (s.workspace_id && (!row || row.stale) && !marked.has(s.id)) {
			bumpDirty(s.id)
			marks.dirty.push({ id: s.id, v: 1 })
		}
	}
	return true
}

/** Read what the stores hold for a dirty session and plan its push. `undefined` when the
 * session has nowhere to go (unsent), `unavailable` when a store could not be read. */
async function planFor(
	session: Session,
	sync: MirrorSyncState | undefined,
	email: string
): Promise<PlannedPush | undefined | 'unavailable'> {
	const chatIds = await listSessionChatIds(session.id, email)
	if (!chatIds) return 'unavailable'
	const chats: ChatSnapshot[] = []
	for (const id of chatIds) {
		const record = await readStoredChat(id, email)
		if (!record) continue
		const imageIds = (await listChatImageIds(id, email)) ?? []
		if (jsonBytes(record) > MAX_CHAT_BYTES) {
			console.warn(`AI session chat ${id} is too large to back up; leaving it out`)
			chats.push({ id, lastModified: record.lastModified, imageIds, omitted: true })
			continue
		}
		chats.push({ id, lastModified: record.lastModified, record, imageIds })
	}
	let artifacts = await readSessionArtifacts(session.id, email)
	if (!artifacts) return 'unavailable'
	if (jsonBytes(artifacts) > MAX_ARTIFACTS_BYTES) {
		console.warn(`AI session ${session.id} artifacts are too large to back up; leaving them out`)
		artifacts = { items: [], versions: [] }
	}
	return planSessionPush({ session, chats, artifacts, sync })
}

interface WorkspaceWork {
	items: { session: Session; v: number; sync?: MirrorSyncState }[]
	/** `key` is the localStorage mark; absent when the removal rides on the sync row.
	 * `storageId` is the storage the backup is in, when a sync row says: a removal is done
	 * only once that storage answered it. */
	removed: { id: string; key?: string; storageId?: string }[]
}

type SendStatus = 'ok' | 'off' | 'refused' | 'abort' | 'transient'

interface WorkspaceOutcome {
	status: SendStatus
	/** The storage and backup generation the server answered from, once it answered. */
	storageId?: string
	generation?: number
	/** Sessions every part of which the server stored. */
	settled: {
		id: string
		v: number
		next: MirrorSyncState
		removeFrom?: string
		carried: boolean
		storageId?: string
		generation?: number
	}[]
	/** Marks with nothing behind them (an unsent draft, a session gone from the store). */
	dropped: { id: string; v: number }[]
	/** Sessions the server holds no head of any more (another device removed the backup):
	 * their rows go stale, so the next flush sends them whole. */
	needsWhole: string[]
	/** Removal marks the server carried out. */
	removedDone: { id: string; key?: string }[]
	/** Some session the server could not store. */
	anyFailed: boolean
	/** Some store could not be read; its marks stay for a later flush. */
	unavailable: boolean
}

/**
 * Plan and send a workspace's sessions one at a time, filling requests of about
 * REQUEST_TARGET_BYTES as it goes, so a first backfill never holds more than one
 * request's worth of records and images at once. Removals go first, in their own
 * request(s).
 */
async function pushWorkspace(
	ws: string,
	work: WorkspaceWork,
	email: string
): Promise<WorkspaceOutcome> {
	const out: WorkspaceOutcome = {
		status: 'ok',
		settled: [],
		dropped: [],
		needsWhole: [],
		removedDone: [],
		anyFailed: false,
		unavailable: false
	}
	// Parts of a session still to be acknowledged, and the sessions the server refused.
	// `complete` once every part of the session has been appended: a session whose first
	// part is still to come when a request fails has nothing behind it yet.
	const attempted = new Map<
		string,
		{
			v: number
			next: MirrorSyncState
			parts: number
			complete: boolean
			removeFrom?: string
			carried: boolean
			storageId?: string
			generation?: number
			/** The storage and generation the last answer for this session came from. */
			answered?: string
		}
	>()
	const failed = new Set<string>()
	const headless = new Set<string>()
	let current: PushBody | undefined
	let size = 0
	let ops = 0

	const send = async (body: PushBody): Promise<SendStatus> => {
		if (getCurrentUserEmail() !== email) return 'abort'
		let res
		try {
			res = await AiService.pushAiSessionBackups({ workspace: ws, requestBody: body })
		} catch (e) {
			const status = statusOf(e)
			// 404: a build without object storage. 403: nothing this token may back up.
			if (status === 404 || status === 403) return 'off'
			if (status === 409) return 'abort'
			// Too large is a fact about the sessions in this body, not the workspace: they
			// stay marked (and retried with backoff), the others go on.
			if (status === 413) {
				console.error('Session backup push too large', e)
				for (const entry of body.sessions) failed.add(entry.id)
				for (const id of body.removed ?? []) failed.add(id)
				return 'ok'
			}
			if (status !== undefined && status < 500 && status !== 429) {
				console.error('Session backup push refused', e)
				return 'refused'
			}
			console.warn('Session backup push failed, retrying later', e)
			return 'transient'
		}
		if (!res.enabled) return 'off'
		out.storageId = res.storage_id
		out.generation = res.backup_generation
		const answered = `${res.storage_id}:${res.backup_generation}`
		const errors = new Set<string>()
		for (const r of res.results) {
			if (r.error) {
				console.warn(`Session backup of ${r.id} failed: ${r.error}`)
				errors.add(r.id)
			} else if (r.needs_head) {
				// The pieces landed but the session has no head there any more: not a
				// failure to back off from, but nothing to settle either.
				headless.add(r.id)
				out.needsWhole.push(r.id)
			}
		}
		for (const entry of body.sessions) {
			const a = attempted.get(entry.id)
			if (!a) continue
			a.parts -= 1
			// Parts answered from different storages sit in different buckets: nothing to
			// settle, the session goes again whole.
			if (a.answered !== undefined && a.answered !== answered) failed.add(entry.id)
			a.answered = answered
			a.storageId = res.storage_id
			a.generation = res.backup_generation
			if (errors.has(entry.id)) failed.add(entry.id)
		}
		for (const id of body.removed ?? []) {
			if (errors.has(id)) failed.add(id)
			else {
				const mark = work.removed.find((r) => r.id === id)
				// Answered from another storage than the one holding the backup: the copy is
				// still there, and the mark waits for that storage to answer again.
				const elsewhere =
					mark?.storageId !== undefined &&
					res.storage_id !== undefined &&
					mark.storageId !== res.storage_id
				if (mark && !elsewhere) out.removedDone.push(mark)
			}
		}
		return 'ok'
	}
	const flushCurrent = async (): Promise<SendStatus> => {
		if (!current) return 'ok'
		const body = current
		current = undefined
		size = 0
		ops = 0
		return send(body)
	}
	const append = async (entry: AISessionBackupPush): Promise<SendStatus> => {
		const bytes = jsonBytes(entry)
		const entryOps = operationsOf(entry)
		if (
			current &&
			(current.sessions.length >= MAX_ENTRIES_PER_REQUEST ||
				ops + entryOps > MAX_OPERATIONS_PER_REQUEST ||
				size + bytes > REQUEST_TARGET_BYTES)
		) {
			const status = await flushCurrent()
			if (status !== 'ok') return status
		}
		// A part the server refused holds the rest of its session back: the head rides on
		// the last part, and would list a session missing a piece.
		if (failed.has(entry.id)) return 'ok'
		current ??= { owner: email, sessions: [] }
		current.sessions.push(entry)
		size += bytes
		ops += entryOps
		const a = attempted.get(entry.id)
		if (a) a.parts += 1
		return 'ok'
	}
	const finish = (status: SendStatus): WorkspaceOutcome => {
		out.status = status
		for (const [id, a] of attempted) {
			if (a.complete && a.parts === 0 && !failed.has(id) && !headless.has(id)) {
				out.settled.push({
					id,
					v: a.v,
					next: a.next,
					removeFrom: a.removeFrom,
					carried: a.carried,
					storageId: a.storageId,
					generation: a.generation
				})
			}
		}
		out.anyFailed = failed.size > 0
		return out
	}

	for (let i = 0; i < work.removed.length; i += MAX_REMOVED_PER_REQUEST) {
		const chunk = work.removed.slice(i, i + MAX_REMOVED_PER_REQUEST)
		const status = await send({ owner: email, sessions: [], removed: chunk.map((r) => r.id) })
		if (status !== 'ok') return finish(status)
	}

	for (const item of work.items) {
		if (getCurrentUserEmail() !== email) return finish('abort')
		const plan = await planFor(item.session, item.sync, email)
		if (plan === 'unavailable') {
			out.unavailable = true
			continue
		}
		if (!plan) {
			out.dropped.push({ id: item.session.id, v: item.v })
			continue
		}
		const nothingToSend = !plan.entry && plan.images.length === 0
		// A move: the copy in the old workspace is filed for removal once this push has
		// landed (see `settled`), never before, so the session is backed up somewhere at
		// every point.
		// A session with nothing to send gets no answer to name its storage: it keeps the
		// one its row has, which the storage check below then judges like any other.
		attempted.set(item.session.id, {
			v: item.v,
			next: plan.next,
			parts: 0,
			complete: nothingToSend,
			removeFrom: plan.removeFrom,
			carried: plan.carried,
			storageId: item.sync?.storageId,
			generation: item.sync?.generation
		})
		if (nothingToSend) continue
		let images: AISessionBackupImage[] = []
		let imagesBytes = 0
		for (const { chat_id, id } of plan.images) {
			const data_url = await readImageDataUrl(id, email)
			// Evicted since the plan was made; the next save of that chat drops the id.
			if (!data_url) continue
			if (
				images.length > 0 &&
				(images.length >= MAX_IMAGES_PER_ENTRY ||
					imagesBytes + data_url.length > REQUEST_TARGET_BYTES)
			) {
				const status = await append({ id: item.session.id, images, partial: true })
				if (status !== 'ok') return finish(status)
				images = []
				imagesBytes = 0
			}
			images.push({ chat_id, id, data_url })
			imagesBytes += data_url.length
		}
		// Every part but the last says so: the server lists a session on the part that
		// completes its entry, never on one an unsent part still follows.
		const parts = plan.entry ? splitEntry(plan.entry, REQUEST_TARGET_BYTES) : []
		if (images.length > 0) {
			const status = await append({
				id: item.session.id,
				images,
				partial: parts.length > 0 || undefined
			})
			if (status !== 'ok') return finish(status)
		}
		for (const [i, part] of parts.entries()) {
			if (failed.has(item.session.id)) break
			const status = await append(i < parts.length - 1 ? { ...part, partial: true } : part)
			if (status !== 'ok') return finish(status)
		}
		attempted.get(item.session.id)!.complete = true
	}
	return finish(await flushCurrent())
}

async function flush(): Promise<void> {
	const email = getCurrentUserEmail()
	if (!email) return
	if (Date.now() < retryAt) {
		clearTimeout(retryTimer)
		retryTimer = setTimeout(runFlush, retryAt - Date.now())
		return
	}
	await withUserLock(email, async () => {
		const marks = readPending()
		// A store that cannot be opened right now (another tab's upgrade in progress) is
		// retried with backoff, as any other unavailable store below.
		if (!(await backfillMarks(marks, email))) {
			backOff()
			return
		}
		// Read once: retired marks are not reclaimed (a mark cannot be deleted without a
		// window in which a bump is lost), so there is one per session ever backed up, and
		// telling them apart from live ones is what lets a flush with nothing to do stop
		// here, before the sessions store.
		const syncRows = new Map((await allSyncRows(email)).map((row) => [row.id, row]))
		// A mark's counter counts with the bumps its row carries (the ones localStorage refused).
		const live = marks.dirty
			.map((d) => ({ id: d.id, v: d.v + (syncRows.get(d.id)?.extraV ?? 0) }))
			.filter((d) => {
				const sync = syncRows.get(d.id)
				return !(sync && !sync.stale && (sync.flushedV ?? -1) >= d.v)
			})
		// A removal whose mark could not be written rides on the sync row instead.
		const removals: { id: string; ws?: string; key?: string }[] = [...marks.removed]
		for (const row of syncRows.values()) {
			if (row.removed && !removals.some((r) => r.id === row.id)) {
				removals.push({ id: row.id, ws: row.ws })
			}
		}
		if (live.length === 0 && removals.length === 0) return
		const stored = await readStoredSessions(email)
		if (!stored) {
			backOff()
			return
		}
		const byId = new Map(stored.map((s) => [s.id, s]))
		const work = new Map<string, WorkspaceWork>()
		const workFor = (ws: string) => {
			let w = work.get(ws)
			if (!w) work.set(ws, (w = { items: [], removed: [] }))
			return w
		}
		const droppedDirty: string[] = []
		const consumedRemoved: string[] = []

		for (const r of removals) {
			const sync = syncRows.get(r.id)
			const ws = r.ws ?? sync?.ws
			// Nowhere to remove it from (an unsent draft): done. A workspace whose backups are
			// off keeps the removal of a session that was backed up, for when they are on
			// again, or the session would come back; one never backed up from here has
			// nothing there, so its mark goes, or a storage-less instance would collect one
			// per deleted session forever.
			if (!ws) {
				if (r.key) consumedRemoved.push(r.key)
			} else if (!wsState.has(ws) || wsState.get(ws) === 'on') {
				workFor(ws).removed.push({ id: r.id, key: r.key, storageId: sync?.storageId })
			} else if (wsState.get(ws) === 'off' && !sync && r.key) consumedRemoved.push(r.key)
		}
		for (const d of live) {
			const session = byId.get(d.id)
			// Gone from the store, so nothing can bump it again; an unsent draft keeps its
			// mark, since it may commit to a workspace while this flush runs.
			if (!session) {
				droppedDirty.push(d.id)
				continue
			}
			if (session.transient || !session.workspace_id) continue
			const state = wsState.get(session.workspace_id)
			// Left in place for a workspace that is off or refused: a move into it must still
			// remember the old copy, and a mark costs one lookup per flush.
			if (state === 'off' || state === 'refused') continue
			const sync = syncRows.get(d.id)
			// A stale row plans like no row at all: the whole session goes again. One naming
			// another workspace still says where the old copy is, stale or not.
			workFor(session.workspace_id).items.push({
				session,
				v: d.v,
				sync: sync?.stale && sync.ws === session.workspace_id ? undefined : sync
			})
		}

		let settledAny = false
		let leftForNext = false
		for (const [ws, w] of work) {
			const out = await pushWorkspace(ws, w, email)
			if (out.status === 'abort') return
			if (out.status === 'off') {
				wsState.set(ws, 'off')
				await staleWorkspaceSync(ws, email)
				// Same rule as the loop above: a removal is worth keeping only for a session
				// that was backed up from here.
				for (const r of w.removed) {
					if (!syncRows.has(r.id) && r.key) consumedRemoved.push(r.key)
				}
				continue
			}
			// Refused stops the workspace for the page, but what the earlier requests of this
			// flush stored is recorded like any other.
			if (out.status === 'refused') wsState.set(ws, 'refused')
			if (out.status === 'transient' || out.anyFailed || out.unavailable) backOff()
			else settledAny = true
			// The new workspace holds a moved session now, so the old copy can go: its removal
			// mark is written before the row that forgets where the old copy was, and a
			// session whose mark could not be written keeps its old row, so the next flush
			// plans the move again rather than orphan the copy.
			const recorded = out.settled.filter((s) => {
				if (s.removeFrom && !addRemoved(s.id, s.removeFrom, false)) return false
				if (s.removeFrom || s.carried) leftForNext = true
				return true
			})
			// A session with deletes carried over stays one bump short of retired, so the
			// next flush sends the rest.
			const written = recorded.map((s) => ({
				...s.next,
				flushedV: s.carried ? s.v - 1 : s.v,
				storageId: s.storageId,
				generation: s.generation
			}))
			await writeSync(written, email)
			// The server names the storage and generation every answer comes from. Rows
			// naming another go stale, the ones written just now included: a session answered
			// from a storage the later answers left behind, or pushed in part on top of a row
			// from another one, has its backup split across buckets the server no longer
			// looks at as a whole.
			if (out.storageId !== undefined) {
				const storageId = out.storageId
				const generation = out.generation ?? 0
				const elsewhere = (row: MirrorSyncState) =>
					row.storageId !== storageId || (row.generation ?? 0) !== generation
				const removed = new Set(out.removedDone.map((r) => r.id))
				const writtenIds = new Set(written.map((row) => row.id))
				const untouched = [...syncRows.values()].filter(
					(row) => !removed.has(row.id) && !writtenIds.has(row.id)
				)
				const foreign = [
					...foreignRows(ws, storageId, generation, untouched),
					...written.filter((row) => {
						const prior = syncRows.get(row.id)
						const partial = prior && !prior.stale && prior.ws === ws && elsewhere(prior)
						return elsewhere(row) || partial
					})
				]
				if (foreign.length > 0) {
					await markStale(foreign, email)
					leftForNext = true
				}
			}
			droppedDirty.push(...out.dropped.map((d) => d.id))
			// The server holds no head of these any more (another device removed the backup):
			// stale rows send them whole next.
			const headless = out.needsWhole
				.map((id) => syncRows.get(id))
				.filter((row): row is MirrorSyncState => row !== undefined && !row.stale)
			if (headless.length > 0) {
				await markStale(headless, email)
				leftForNext = true
			}
			for (const r of out.removedDone) {
				// The row describes this workspace's copy only; a session that moved on keeps
				// the row its new workspace wrote.
				if ((await readSync(r.id, email))?.ws === ws) await deleteSync([r.id], email)
				if (r.key) consumedRemoved.push(r.key)
			}
		}
		if (settledAny && Date.now() >= retryAt) retryMs = RETRY_MIN_MS
		if (getCurrentUserEmail() !== email) return
		for (const id of droppedDirty) dropDirty(id)
		for (const key of consumedRemoved) removeKey(key)
		// Whatever is still marked is either waiting on the backoff timer, on a write that
		// scheduled its own flush, or on a workspace that is off or refused for the page;
		// none of it wants another flush in 15 s. What this flush left for the next one
		// does: a moved session's removal from its old workspace, carried-over deletes, or
		// the sessions of a storage the server no longer answers from.
		if (leftForNext) scheduleFlush()
	})
}

// --- Restore ---

function asRecord(value: unknown): Record<string, unknown> | undefined {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: undefined
}

/** Turn one pulled backup into store rows, dropping anything that does not name this
 * session: the bucket is written by the server from validated ids, but a record is
 * still data from outside this browser. */
function unpackBackup(
	ws: string,
	backup: AISessionBackup,
	updatedAt: number,
	storageId: string | undefined,
	generation: number | undefined,
	earlierChats: Iterable<string> = []
):
	| {
			session: Session
			chats: StoredChat[]
			images: RestoredImage[]
			artifacts: { items: PersistedArtifact[]; versions: ArtifactVersion[] }
			sync: MirrorSyncState
	  }
	| undefined {
	const head = asRecord(backup.head)
	if (!head || head.id !== backup.id || head.workspace_id !== ws) return undefined
	if (typeof head.createdAt !== 'number') return undefined
	const chats: StoredChat[] = []
	for (const c of backup.chats) {
		const record = asRecord(c.record)
		if (
			!record ||
			record.id !== c.id ||
			record.sessionId !== backup.id ||
			!Array.isArray(record.actualMessages) ||
			!Array.isArray(record.displayMessages) ||
			typeof record.lastModified !== 'number'
		) {
			continue
		}
		chats.push(record as unknown as StoredChat)
	}
	// An image's chat may have come on an earlier page of the session.
	const chatIds = new Set([...chats.map((c) => c.id), ...earlierChats])
	const images: RestoredImage[] = backup.images
		.filter((i) => chatIds.has(i.chat_id) && typeof i.data_url === 'string')
		.map((i) => ({ id: i.id, chatId: i.chat_id, dataUrl: i.data_url }))
	const artifactsRecord = asRecord(backup.artifacts)
	const items = (
		Array.isArray(artifactsRecord?.items) ? (artifactsRecord.items as PersistedArtifact[]) : []
	).filter((i) => asRecord(i)?.sessionId === backup.id && typeof i.id === 'string')
	const itemIds = new Set(items.map((i) => i.id))
	const versions = (
		Array.isArray(artifactsRecord?.versions) ? (artifactsRecord.versions as ArtifactVersion[]) : []
	).filter((v) => asRecord(v) && itemIds.has(v.artifactId) && typeof v.key === 'string')
	const active = chats.find((c) => c.id === head.chatId)
	const session: Session = {
		...(head as unknown as Session),
		name: '',
		// Everything in the backup has been read here: no unread badge, and the last
		// activity is the backup's own time.
		lastSeenCount: active?.displayMessages.length ?? 0,
		lastActivityAt: updatedAt
	}
	const sync: MirrorSyncState = {
		id: backup.id,
		ws,
		head: headSig(session),
		chats: Object.fromEntries(chats.map((c) => [c.id, c.lastModified])),
		images: Object.fromEntries(images.map((i) => [i.id, i.chatId])),
		artifacts: artifactsFingerprint({ items, versions }),
		storageId,
		generation
	}
	return { session, chats, images, artifacts: { items, versions }, sync }
}

async function restoreWorkspace(ws: string, email: string): Promise<void> {
	let listing
	try {
		listing = await AiService.listAiSessionBackups({ workspace: ws })
	} catch (e) {
		const status = statusOf(e)
		if (status === 404 || status === 403) wsState.set(ws, 'off')
		else console.warn('Could not list session backups', e)
		return
	}
	if (!listing.enabled) {
		wsState.set(ws, 'off')
		return
	}
	if (!wsState.has(ws)) wsState.set(ws, 'on')
	const rows = await allSyncRows(email)
	const foreign =
		listing.storage_id === undefined
			? []
			: foreignRows(ws, listing.storage_id, listing.backup_generation ?? 0, rows)
	if (foreign.length > 0) {
		await markStale(foreign, email)
		scheduleFlush()
	}
	const local = new Set<string>()
	for (const s of (await readStoredSessions(email)) ?? []) local.add(s.id)
	for (const s of sessionState.sessions) local.add(s.id)
	// A removal names the workspace it is for: a session that moved here from another one
	// still has that one's removal pending, and is ours to restore.
	for (const r of readPending().removed) if (r.ws === ws) local.add(r.id)
	for (const row of rows) if (row.removed && row.ws === ws) local.add(row.id)
	const candidates = listing.sessions
		.filter((s) => !local.has(s.id) && !isSessionTombstoned(s.id))
		.slice(0, RESTORE_MAX)
	const updatedAt = new Map<string, number>(candidates.map((s) => [s.id, Date.parse(s.updated_at)]))
	let ids = candidates.map((s) => s.id)
	let restored = 0
	// A session that did not fit one answer whole comes in pages, kept here until the last
	// one: importing a page alone would leave a session the next restore takes for whole.
	type Pieces = {
		chats: Set<string>
		images: Set<string>
		items: Set<string>
		versions: Set<string>
	}
	type Staged = {
		session: Session
		sync: MirrorSyncState
		/** Everything the pages so far wrote for the session. */
		pieces: Pieces
		/** The listing fingerprint the pages so far were answered with. */
		listing?: string
	}
	const staged = new Map<string, Staged>()
	const noPieces = (): Pieces => ({
		chats: new Set(),
		images: new Set(),
		items: new Set(),
		versions: new Set()
	})
	const union = (a: Pieces, b: Pieces): Pieces => ({
		chats: new Set([...a.chats, ...b.chats]),
		images: new Set([...a.images, ...b.images]),
		items: new Set([...a.items, ...b.items]),
		versions: new Set([...a.versions, ...b.versions])
	})
	// What earlier attempts (a restore cut short, a start over) wrote for a session that has
	// no record yet, from their staging rows: the pieces of it the backup no longer has go
	// before the record lands, or a later flush would push them back. Ids, never clocks.
	const earlierStaging = new Map<string, Pieces>()
	for (const row of rows) {
		if (row.staging) {
			earlierStaging.set(row.id, {
				chats: new Set(row.staging.chats),
				images: new Set(row.staging.images),
				items: new Set(row.staging.items),
				versions: new Set(row.staging.versions)
			})
		}
	}
	// A session whose backup moved between two of its pages starts over, a few times.
	const restarts = new Map<string, number>()
	const MAX_RESTARTS = 3
	const resumes: AISessionBackupCursor[] = []
	while (ids.length > 0 || resumes.length > 0) {
		if (getCurrentUserEmail() !== email) return
		const resume = resumes.shift()
		const batch = resume ? [resume.id] : ids.slice(0, PULL_BATCH)
		if (!resume) ids = ids.slice(PULL_BATCH)
		let pulled
		try {
			pulled = await AiService.pullAiSessionBackups({
				workspace: ws,
				requestBody: { ids: batch, resume }
			})
		} catch (e) {
			console.warn('Could not pull session backups', e)
			return
		}
		if (!pulled.enabled) {
			wsState.set(ws, 'off')
			return
		}
		// Ask again for what did not fit, one at a time so each answer is as small as can be.
		for (const id of pulled.deferred) if (!ids.includes(id)) ids.unshift(id)
		// A session's pieces land before its record, page by page (the writes are absent-only,
		// so a restore cut short leaves nothing a later one cannot finish), and the record,
		// which is what makes the session visible, only with the last page; a session whose
		// pieces could not be written is left for the next restore, since recording it now
		// would let the next flush push its half-empty local state over the backup. What a
		// page leaves for the next is the sync row being assembled, never its pieces.
		const ready: Staged[] = []
		for (const b of pulled.sessions) {
			const earlier = staged.get(b.id)
			staged.delete(b.id)
			// Imported by another tab meanwhile (the lock keeps that from happening where Web
			// Locks exist): its pieces are not ours to write over any more.
			if ((await readStoredSessions(email))?.some((s) => s.id === b.id)) continue
			// The backup moved between two pages (a chat sorting before the cursor would be
			// missed): the pages so far do not belong together, the session starts over.
			if (earlier?.listing !== undefined && b.listing !== earlier.listing) {
				const n = (restarts.get(b.id) ?? 0) + 1
				restarts.set(b.id, n)
				earlierStaging.set(b.id, union(earlierStaging.get(b.id) ?? noPieces(), earlier.pieces))
				if (n < MAX_RESTARTS) ids.unshift(b.id)
				else console.warn(`Session backup ${b.id} kept changing while restoring; left for later`)
				continue
			}
			const u = unpackBackup(
				ws,
				b,
				updatedAt.get(b.id) ?? Date.now(),
				pulled.storage_id,
				pulled.backup_generation,
				Object.keys(earlier?.sync.chats ?? {})
			)
			if (!u) continue
			const written: Pieces = {
				chats: new Set(u.chats.map((c) => c.id)),
				images: new Set(u.images.map((i) => i.id)),
				items: new Set(u.artifacts.items.map((i) => i.id)),
				versions: new Set(u.artifacts.versions.map((v) => v.key))
			}
			const merged: Staged = earlier
				? {
						session: {
							...u.session,
							lastSeenCount: Math.max(
								earlier.session.lastSeenCount ?? 0,
								u.session.lastSeenCount ?? 0
							)
						},
						sync: {
							...u.sync,
							chats: { ...earlier.sync.chats, ...u.sync.chats },
							images: { ...earlier.sync.images, ...u.sync.images },
							artifacts: b.artifacts !== undefined ? u.sync.artifacts : earlier.sync.artifacts
						},
						pieces: union(earlier.pieces, written)
					}
				: { session: u.session, sync: u.sync, pieces: written }
			merged.listing = b.listing
			// The staging row goes before the pieces, and outlives a restore cut short: the
			// next one reads it to know what to delete. The record replaces it.
			const stagingPieces = union(earlierStaging.get(b.id) ?? noPieces(), merged.pieces)
			await writeSync(
				[
					{
						id: b.id,
						ws,
						head: '',
						chats: {},
						images: {},
						staging: {
							chats: [...stagingPieces.chats],
							images: [...stagingPieces.images],
							items: [...stagingPieces.items],
							versions: [...stagingPieces.versions]
						}
					}
				],
				email
			)
			// Written over whatever is there: the session is absent locally, so its pieces
			// can only be what an earlier restore staged before it was cut short, and the
			// backup may have moved on since.
			try {
				if (!(await importArtifacts(u.artifacts.items, u.artifacts.versions, email, true))) continue
				if (!(await importStoredChats(u.chats, u.images, email, true))) continue
			} catch (e) {
				console.error(`Could not restore session ${u.session.id}`, e)
				continue
			}
			if (b.next) {
				staged.set(b.id, merged)
				resumes.push(b.next)
				continue
			}
			ready.push(merged)
		}
		if (ready.length === 0) continue
		for (const r of ready) {
			const prior = earlierStaging.get(r.session.id)
			if (!prior) continue
			const gone = (was: Set<string>, now: Set<string>) =>
				new Set([...was].filter((id) => !now.has(id)))
			await pruneSessionChats(
				r.session.id,
				gone(prior.chats, r.pieces.chats),
				gone(prior.images, r.pieces.images),
				email
			)
			await pruneSessionArtifacts(
				r.session.id,
				gone(prior.items, r.pieces.items),
				gone(prior.versions, r.pieces.versions),
				email
			)
			earlierStaging.delete(r.session.id)
		}
		const imported = new Set(
			await importSessions(
				ready.map((r) => r.session),
				email
			)
		)
		await writeSync(
			ready.filter((r) => imported.has(r.session.id)).map((r) => r.sync),
			email
		)
		restored += imported.size
	}
	if (restored > 0) logFeatureUsage('ai_session', 'restored', { value: restored })
}

/**
 * Restore the sessions of the workspace family the user is looking at (the workspace and
 * its forks), once per workspace per page load. Sessions that exist here are never
 * touched; only ones this browser lacks are brought back.
 */
export function restoreSessionBackups(currentWorkspace: string): void {
	if (!BROWSER) return
	const email = getCurrentUserEmail()
	if (!email) return
	const all = get(userWorkspaces)
	const root = workspaceRootId(currentWorkspace, all) ?? currentWorkspace
	const family = new Set<string>([currentWorkspace])
	for (const w of all) if ((workspaceRootId(w.id, all) ?? w.id) === root) family.add(w.id)
	for (const ws of family) {
		if (restoredWorkspaces.has(ws) || wsState.get(ws) === 'off') continue
		restoredWorkspaces.add(ws)
		void enqueue(() => withUserLock(email, () => restoreWorkspace(ws, email), true))
	}
}

// --- Wiring ---

if (BROWSER) {
	onMirrorSignal((signal) => {
		// A mark for another user waits for that user's next load.
		const mine = !signal.email || signal.email === getCurrentUserEmail()
		if (signal.kind === 'dirty') {
			if (bumpDirty(signal.sessionId, signal.email)) {
				if (mine) scheduleFlush()
			} else {
				if (mine) unwritableMarks.add(signal.sessionId)
				void bumpViaSyncRow(signal.sessionId, signal.email).finally(() => {
					if (mine) scheduleFlush()
				})
			}
			return
		}
		if (!addRemoved(signal.sessionId, signal.workspaceId, true, signal.email)) {
			void removeViaSyncRow(signal.sessionId, signal.workspaceId, signal.email)
		}
		if (mine) scheduleFlush()
	})
	onUserChange((email) => {
		clearTimers()
		clearTimeout(retryTimer)
		retryAt = 0
		retryMs = RETRY_MIN_MS
		wsState.clear()
		restoredWorkspaces.clear()
		backfilled = false
		unwritableMarks.clear()
		if (email) setTimeout(runFlush, STARTUP_DELAY_MS)
	})
	// A tab going to the background may not come back: carry what it has now.
	if (typeof document !== 'undefined') {
		document.addEventListener('visibilitychange', () => {
			if (document.visibilityState === 'hidden') runFlush()
		})
	}
}

/** Test-only: run a flush now, outside the timers. */
export function __flushForTesting(): Promise<void | undefined> {
	return enqueue(flush)
}

/** Test-only: what the sync table holds for the user. */
export async function __syncRowsForTesting(email: string): Promise<MirrorSyncState[]> {
	return allSyncRows(email)
}

/** Test-only: wait for whatever flush or restore is queued. */
export function __settleForTesting(): Promise<void> {
	return enqueue(async () => {})
}

/** Test-only: forget every page-lifetime decision, and let go of the sync store so the
 * next open lands in the test's fresh IndexedDB rather than the cached connection. */
export function __resetMirrorForTesting(): void {
	syncDbh.close()
	clearTimers()
	clearTimeout(retryTimer)
	retryAt = 0
	retryMs = RETRY_MIN_MS
	wsState.clear()
	restoredWorkspaces.clear()
	backfilled = false
	unwritableMarks.clear()
}
