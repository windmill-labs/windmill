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
import { type DBSchema } from 'idb'
import {
	AiService,
	ApiError,
	type AISessionBackup,
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
	listChatImageIds,
	listSessionChatIds,
	readImageDataUrl,
	readStoredChat,
	type RestoredImage,
	type StoredChat
} from '../copilot/chat/HistoryManager.svelte'
import {
	importArtifacts,
	readSessionArtifacts,
	type ArtifactVersion,
	type PersistedArtifact
} from '../copilot/chat/artifacts/artifactsDB'
import {
	artifactsFingerprint,
	headSig,
	jsonBytes,
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
/** The server's caps per request. */
const MAX_ENTRIES_PER_REQUEST = 100
const MAX_REMOVED_PER_REQUEST = 200
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

const syncDbh = userScopedDb<MirrorSchema>(SYNC_DB, {
	version: 1,
	upgrade(db) {
		if (!db.objectStoreNames.contains('sync')) db.createObjectStore('sync', { keyPath: 'id' })
	}
})

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
	return marks
}

function hasPending(): boolean {
	const marks = readPending()
	return marks.dirty.length > 0 || marks.removed.length > 0
}

function bumpDirty(sessionId: string): void {
	const base = pendingBase()
	if (!base) return
	try {
		const key = dirtyKey(base, sessionId)
		const v = Number(localStorage.getItem(key) ?? '0')
		localStorage.setItem(key, String((Number.isFinite(v) ? v : 0) + 1))
	} catch (e) {
		console.error('Could not persist session backup mark', e)
	}
}

/** Clear a dirty mark, unless it was bumped since the flush read it. */
function clearDirty(sessionId: string, v: number): void {
	const base = pendingBase()
	if (!base) return
	try {
		const key = dirtyKey(base, sessionId)
		if (Number(localStorage.getItem(key)) === v) localStorage.removeItem(key)
	} catch {}
}

function addRemoved(sessionId: string, ws: string | undefined, dropDirty: boolean): void {
	const base = pendingBase()
	if (!base) return
	try {
		if (dropDirty) localStorage.removeItem(dirtyKey(base, sessionId))
		localStorage.setItem(removedKey(base, sessionId, ws), '1')
	} catch (e) {
		console.error('Could not persist session backup mark', e)
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
async function withUserLock(email: string, fn: () => Promise<void>): Promise<void> {
	const locks =
		typeof navigator === 'undefined' ? undefined : (navigator as { locks?: LockManager }).locks
	if (!locks) return fn()
	await locks.request(`wm-ai-sessions-mirror::${email}`, { ifAvailable: true }, async (lock) => {
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

async function writeSync(states: MirrorSyncState[], email: string): Promise<void> {
	const db = await syncDb(email)
	if (!db || states.length === 0) return
	const tx = db.transaction('sync', 'readwrite')
	for (const s of states) await tx.store.put(s)
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
	removed: { id: string; key: string }[]
}

type SendStatus = 'ok' | 'off' | 'refused' | 'abort' | 'transient'

interface WorkspaceOutcome {
	status: SendStatus
	/** Sessions every part of which the server stored. */
	settled: { id: string; v: number; next: MirrorSyncState }[]
	/** Marks with nothing behind them (an unsent draft, a session gone from the store). */
	dropped: { id: string; v: number }[]
	/** Removal marks the server carried out. */
	removedDone: { id: string; key: string }[]
	/** Some session the server could not store. */
	anyFailed: boolean
	/** Some store could not be read; its marks stay for a later flush. */
	unavailable: boolean
	/** A session moved workspace and left a removal mark for the old one behind. */
	movedAny: boolean
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
		removedDone: [],
		anyFailed: false,
		unavailable: false,
		movedAny: false
	}
	// Parts of a session still to be acknowledged, and the sessions the server refused.
	// `complete` once every part of the session has been appended: a session whose first
	// part is still to come when a request fails has nothing behind it yet.
	const attempted = new Map<
		string,
		{ v: number; next: MirrorSyncState; parts: number; complete: boolean }
	>()
	const failed = new Set<string>()
	let current: PushBody | undefined
	let size = 0

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
		const errors = new Set<string>()
		for (const r of res.results) {
			if (r.error) {
				console.warn(`Session backup of ${r.id} failed: ${r.error}`)
				errors.add(r.id)
			}
		}
		for (const entry of body.sessions) {
			const a = attempted.get(entry.id)
			if (!a) continue
			a.parts -= 1
			if (errors.has(entry.id)) failed.add(entry.id)
		}
		for (const id of body.removed ?? []) {
			if (errors.has(id)) failed.add(id)
			else {
				const mark = work.removed.find((r) => r.id === id)
				if (mark) out.removedDone.push(mark)
			}
		}
		return 'ok'
	}
	const flushCurrent = async (): Promise<SendStatus> => {
		if (!current) return 'ok'
		const body = current
		current = undefined
		size = 0
		return send(body)
	}
	const append = async (entry: AISessionBackupPush): Promise<SendStatus> => {
		const bytes = jsonBytes(entry)
		if (
			current &&
			(current.sessions.length >= MAX_ENTRIES_PER_REQUEST || size + bytes > REQUEST_TARGET_BYTES)
		) {
			const status = await flushCurrent()
			if (status !== 'ok') return status
		}
		current ??= { owner: email, sessions: [] }
		current.sessions.push(entry)
		size += bytes
		const a = attempted.get(entry.id)
		if (a) a.parts += 1
		return 'ok'
	}
	const finish = (status: SendStatus): WorkspaceOutcome => {
		out.status = status
		for (const [id, a] of attempted) {
			if (a.complete && a.parts === 0 && !failed.has(id)) {
				out.settled.push({ id, v: a.v, next: a.next })
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
		// A move: the copy in the old workspace goes on its own mark, so it is retried on
		// its own if this push lands and its removal does not.
		if (plan.removeFrom && wsState.get(plan.removeFrom) !== 'off') {
			addRemoved(item.session.id, plan.removeFrom, false)
			out.movedAny = true
		}
		const nothingToSend = !plan.entry && plan.images.length === 0
		attempted.set(item.session.id, {
			v: item.v,
			next: plan.next,
			parts: 0,
			complete: nothingToSend
		})
		if (nothingToSend) continue
		let images: AISessionBackupImage[] = []
		let imagesBytes = 0
		for (const { chat_id, id } of plan.images) {
			const data_url = await readImageDataUrl(id, email)
			// Evicted since the plan was made; the next save of that chat drops the id.
			if (!data_url) continue
			if (images.length > 0 && imagesBytes + data_url.length > REQUEST_TARGET_BYTES) {
				const status = await append({ id: item.session.id, images })
				if (status !== 'ok') return finish(status)
				images = []
				imagesBytes = 0
			}
			images.push({ chat_id, id, data_url })
			imagesBytes += data_url.length
		}
		if (images.length > 0) {
			const status = await append({ id: item.session.id, images })
			if (status !== 'ok') return finish(status)
		}
		for (const part of plan.entry ? splitEntry(plan.entry, REQUEST_TARGET_BYTES) : []) {
			const status = await append(part)
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
		if (marks.dirty.length === 0 && marks.removed.length === 0) return
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
		const consumedDirty: { id: string; v: number }[] = []
		const consumedRemoved: string[] = []

		for (const r of marks.removed) {
			const sync = await readSync(r.id, email)
			const ws = r.ws ?? sync?.ws
			// Nowhere to remove it from (an unsent draft): done. A workspace whose backups are
			// off keeps the removal of a session that was backed up, for when they are on
			// again, or the session would come back; one never backed up from here has
			// nothing there, so its mark goes, or a storage-less instance would collect one
			// per deleted session forever.
			if (!ws) consumedRemoved.push(r.key)
			else if (!wsState.has(ws) || wsState.get(ws) === 'on') {
				workFor(ws).removed.push({ id: r.id, key: r.key })
			} else if (wsState.get(ws) === 'off' && !sync) consumedRemoved.push(r.key)
		}
		for (const d of marks.dirty) {
			const session = byId.get(d.id)
			if (!session || session.transient || !session.workspace_id) {
				consumedDirty.push(d)
				continue
			}
			const state = wsState.get(session.workspace_id)
			if (state === 'off') consumedDirty.push(d)
			else if (state !== 'refused') {
				// A stale row plans like no row at all: the whole session goes again.
				const sync = await readSync(d.id, email)
				workFor(session.workspace_id).items.push({
					session,
					v: d.v,
					sync: sync?.stale ? undefined : sync
				})
			}
		}

		let settledAny = false
		let movedAny = false
		for (const [ws, w] of work) {
			const out = await pushWorkspace(ws, w, email)
			movedAny ||= out.movedAny
			if (out.status === 'abort') return
			if (out.status === 'off') {
				wsState.set(ws, 'off')
				await staleWorkspaceSync(ws, email)
				for (const item of w.items) consumedDirty.push({ id: item.session.id, v: item.v })
				// Same rule as the loop above: a removal is worth keeping only for a session
				// that was backed up from here.
				for (const r of w.removed) {
					if (!(await readSync(r.id, email))) consumedRemoved.push(r.key)
				}
				continue
			}
			if (out.status === 'refused') {
				wsState.set(ws, 'refused')
				continue
			}
			if (out.status === 'transient' || out.anyFailed || out.unavailable) backOff()
			else settledAny = true
			await writeSync(
				out.settled.map((s) => s.next),
				email
			)
			consumedDirty.push(...out.settled, ...out.dropped)
			for (const r of out.removedDone) {
				// The row describes this workspace's copy only; a session that moved on keeps
				// the row its new workspace wrote.
				if ((await readSync(r.id, email))?.ws === ws) await deleteSync([r.id], email)
				consumedRemoved.push(r.key)
			}
		}
		if (settledAny && Date.now() >= retryAt) retryMs = RETRY_MIN_MS
		if (getCurrentUserEmail() !== email) return
		for (const { id, v } of consumedDirty) clearDirty(id, v)
		for (const key of consumedRemoved) removeKey(key)
		// Whatever is still marked is either waiting on the backoff timer, on a write that
		// scheduled its own flush, or on a workspace that is off or refused for the page;
		// none of it wants another flush in 15 s. The one mark this flush wrote itself, a
		// moved session's removal from its old workspace, does.
		if (movedAny) scheduleFlush()
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
	updatedAt: number
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
	const chatIds = new Set(chats.map((c) => c.id))
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
		artifacts: artifactsFingerprint({ items, versions })
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
	const local = new Set<string>()
	for (const s of (await readStoredSessions(email)) ?? []) local.add(s.id)
	for (const s of sessionState.sessions) local.add(s.id)
	for (const r of readPending().removed) local.add(r.id)
	const candidates = listing.sessions
		.filter((s) => !local.has(s.id) && !isSessionTombstoned(s.id))
		.slice(0, RESTORE_MAX)
	const updatedAt = new Map<string, number>(candidates.map((s) => [s.id, Date.parse(s.updated_at)]))
	let ids = candidates.map((s) => s.id)
	let restored = 0
	while (ids.length > 0) {
		if (getCurrentUserEmail() !== email) return
		const batch = ids.slice(0, PULL_BATCH)
		ids = ids.slice(PULL_BATCH)
		let pulled
		try {
			pulled = await AiService.pullAiSessionBackups({ workspace: ws, requestBody: { ids: batch } })
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
		const unpacked = pulled.sessions
			.map((b) => unpackBackup(ws, b, updatedAt.get(b.id) ?? Date.now()))
			.filter((u) => u !== undefined)
		// A session's pieces land before its record, and a session whose pieces could not be
		// written is left for the next restore: recording it now would let the next flush
		// push its half-empty local state over the backup.
		const ready: typeof unpacked = []
		for (const u of unpacked) {
			try {
				if (!(await importArtifacts(u.artifacts.items, u.artifacts.versions, email))) continue
				if (!(await importStoredChats(u.chats, u.images, email))) continue
				ready.push(u)
			} catch (e) {
				console.error(`Could not restore session ${u.session.id}`, e)
			}
		}
		if (ready.length === 0) continue
		const imported = new Set(
			await importSessions(
				ready.map((u) => u.session),
				email
			)
		)
		await writeSync(
			ready.filter((u) => imported.has(u.session.id)).map((u) => u.sync),
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
		void enqueue(() => restoreWorkspace(ws, email))
	}
}

// --- Wiring ---

if (BROWSER) {
	onMirrorSignal((signal) => {
		if (signal.kind === 'dirty') bumpDirty(signal.sessionId)
		else addRemoved(signal.sessionId, signal.workspaceId, true)
		scheduleFlush()
	})
	onUserChange((email) => {
		clearTimers()
		clearTimeout(retryTimer)
		retryAt = 0
		retryMs = RETRY_MIN_MS
		wsState.clear()
		restoredWorkspaces.clear()
		backfilled = false
		if (email) setTimeout(runFlush, STARTUP_DELAY_MS)
	})
	// A tab going to the background may not come back: carry what it has now.
	if (typeof document !== 'undefined') {
		document.addEventListener('visibilitychange', () => {
			if (document.visibilityState === 'hidden' && hasPending()) runFlush()
		})
	}
}

/** Test-only: run a flush now, outside the timers. */
export function __flushForTesting(): Promise<void | undefined> {
	return enqueue(flush)
}

/** Test-only: wait for whatever flush or restore is queued. */
export function __settleForTesting(): Promise<void> {
	return enqueue(async () => {})
}

/** Test-only: forget every page-lifetime decision. */
export function __resetMirrorForTesting(): void {
	clearTimers()
	clearTimeout(retryTimer)
	retryAt = 0
	retryMs = RETRY_MIN_MS
	wsState.clear()
	restoredWorkspaces.clear()
	backfilled = false
}
