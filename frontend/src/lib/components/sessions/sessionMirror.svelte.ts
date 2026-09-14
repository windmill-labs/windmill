// Lazily backs the browser's AI sessions up to their workspace's object storage, and
// restores the ones this browser does not have.
//
// IndexedDB stays the store every write lands in; the funnels there only mark a session
// dirty (sessionMirrorSignal). A flush runs once the marks have been quiet for a while,
// bounded by a maximum delay so a long turn still gets backed up part-way, and sends one
// batched request per workspace carrying only the pieces whose marker moved
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
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
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
	type ChatSnapshot,
	type MirrorSyncState,
	type PlannedPush
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
/** A chat beyond this is left out of the backup rather than sent. */
const MAX_CHAT_BYTES = 24 * 1024 * 1024
const PULL_BATCH = 5
/** Newest sessions restored per workspace: every visible session gets a runtime, and each
 * runtime's history load reads the whole chat store. */
const RESTORE_MAX = 50
const PENDING_KEY = 'windmill_sessions_mirror_pending'
const SYNC_DB = 'windmill-sessions-mirror'

interface DirtyMark {
	/** Bumped on every mark, so a flush clears only marks it has fully carried. */
	v: number
	chats: string[]
}

interface PendingMarks {
	dirty: Record<string, DirtyMark>
	removed: { id: string; ws?: string }[]
}

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

// --- Pending marks (localStorage, shared by every tab of the user) ---

function emptyMarks(): PendingMarks {
	return { dirty: {}, removed: [] }
}

function readPending(): PendingMarks {
	const key = scopedKey(PENDING_KEY)
	if (!key) return emptyMarks()
	try {
		const raw = getLocalSetting(key)
		if (!raw) return emptyMarks()
		const parsed = JSON.parse(raw)
		return {
			dirty: typeof parsed?.dirty === 'object' && parsed.dirty ? parsed.dirty : {},
			removed: Array.isArray(parsed?.removed) ? parsed.removed : []
		}
	} catch {
		return emptyMarks()
	}
}

function writePending(marks: PendingMarks): void {
	const key = scopedKey(PENDING_KEY)
	if (!key) return
	const empty = Object.keys(marks.dirty).length === 0 && marks.removed.length === 0
	try {
		storeLocalSetting(key, empty ? undefined : JSON.stringify(marks))
	} catch (e) {
		console.error('Could not persist session backup marks', e)
	}
}

function hasPending(marks = readPending()): boolean {
	return Object.keys(marks.dirty).length > 0 || marks.removed.length > 0
}

function addDirty(marks: PendingMarks, sessionId: string, chatId?: string): void {
	const mark = marks.dirty[sessionId] ?? { v: 0, chats: [] }
	mark.v += 1
	if (chatId && !mark.chats.includes(chatId)) mark.chats.push(chatId)
	marks.dirty[sessionId] = mark
}

// --- Scheduling ---

let quietTimer: ReturnType<typeof setTimeout> | undefined
let maxTimer: ReturnType<typeof setTimeout> | undefined
let retryTimer: ReturnType<typeof setTimeout> | undefined
let retryAt = 0
let retryMs = RETRY_MIN_MS
/** Workspaces whose storage answered this page load. */
const wsState = new Map<string, 'on' | 'off'>()
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
	})
}

// --- Flush ---

function disableWorkspace(ws: string): void {
	wsState.set(ws, 'off')
}

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

/** Forget every push into a workspace whose storage is gone, so a storage configured
 * later gets a full backfill instead of nothing. */
async function forgetWorkspaceSync(ws: string, email: string): Promise<void> {
	const db = await syncDb(email)
	if (!db) return
	const gone = (await db.getAll('sync')).filter((s) => s.ws === ws).map((s) => s.id)
	await deleteSync(gone, email)
}

/** Sessions this browser has backed up nothing of yet: everything committed to a
 * workspace and not yet in the sync table gets a mark, once per page load. */
async function backfillMarks(marks: PendingMarks, email: string): Promise<void> {
	if (backfilled) return
	backfilled = true
	const sessions = await readStoredSessions(email)
	const db = await syncDb(email)
	if (!sessions || !db) return
	const known = new Set((await db.getAllKeys('sync')).map(String))
	for (const s of sessions) {
		if (s.workspace_id && !known.has(s.id) && !marks.dirty[s.id]) addDirty(marks, s.id)
	}
}

/** Read what the stores hold for a dirty session and plan its push. `undefined` when the
 * session is gone, unsent, or its store is unavailable. */
async function planFor(
	session: Session,
	mark: DirtyMark,
	sync: MirrorSyncState | undefined,
	email: string
): Promise<PlannedPush | undefined> {
	const chatIds = await listSessionChatIds(session.id, email)
	if (!chatIds) return undefined
	const prev = sync?.ws === session.workspace_id ? sync : undefined
	const chats: ChatSnapshot[] = []
	for (const id of chatIds) {
		// A chat the marks did not name and the last push carried has not changed: its
		// record and image set are exactly what was pushed, so neither is read again.
		if (prev && prev.chats[id] !== undefined && !mark.chats.includes(id)) {
			chats.push({
				id,
				lastModified: prev.chats[id],
				imageIds: Object.entries(prev.images)
					.filter(([, chatId]) => chatId === id)
					.map(([imageId]) => imageId)
			})
			continue
		}
		const record = await readStoredChat(id, email)
		if (!record) continue
		const imageIds = (await listChatImageIds(id, email)) ?? []
		if (jsonBytes(record) > MAX_CHAT_BYTES) {
			console.warn(`AI session chat ${id} is too large to back up; leaving it out`)
			chats.push({ id, lastModified: record.lastModified, imageIds })
			continue
		}
		chats.push({ id, lastModified: record.lastModified, record, imageIds })
	}
	const artifacts = await readSessionArtifacts(session.id, email)
	if (!artifacts) return undefined
	return planSessionPush({ session, chats, artifacts, sync })
}

type PushBody = { owner: string; sessions: AISessionBackupPush[]; removed?: string[] }

/**
 * Pack the workspace's entries into requests of about REQUEST_TARGET_BYTES. Images go
 * first, as entries of their own: the server needs nothing else to store one, and a
 * session's images alone can outweigh a whole request.
 */
function packRequests(
	email: string,
	imageEntries: AISessionBackupPush[],
	entries: AISessionBackupPush[],
	removed: string[]
): PushBody[] {
	const requests: PushBody[] = []
	let current: PushBody | undefined
	let size = 0
	const add = (entry: AISessionBackupPush) => {
		const bytes = jsonBytes(entry)
		if (!current || (size > 0 && size + bytes > REQUEST_TARGET_BYTES)) {
			current = { owner: email, sessions: [] }
			requests.push(current)
			size = 0
		}
		current.sessions.push(entry)
		size += bytes
	}
	for (const entry of imageEntries) add(entry)
	for (const entry of entries) add(entry)
	if (removed.length > 0) {
		if (requests.length === 0) requests.push({ owner: email, sessions: [] })
		requests[0].removed = removed
	}
	return requests
}

/** Load a plan's images and split them into request-sized entries. */
async function imageEntries(plan: PlannedPush, email: string): Promise<AISessionBackupPush[]> {
	const entries: AISessionBackupPush[] = []
	let current: AISessionBackupImage[] = []
	let size = 0
	for (const { chat_id, id } of plan.images) {
		const data_url = await readImageDataUrl(id, email)
		// Evicted since the plan was made; the next save of that chat drops the id.
		if (!data_url) continue
		if (size > 0 && size + data_url.length > REQUEST_TARGET_BYTES) {
			entries.push({ id: plan.next.id, images: current })
			current = []
			size = 0
		}
		current.push({ chat_id, id, data_url })
		size += data_url.length
	}
	if (current.length > 0) entries.push({ id: plan.next.id, images: current })
	return entries
}

interface WorkspaceWork {
	plans: { plan: PlannedPush; mark: DirtyMark }[]
	removed: string[]
}

/** Outcome per session id: absent means the workspace's requests failed as a whole. */
type PushOutcome = { failed: Set<string>; disabled: boolean; abort: boolean } | undefined

async function pushWorkspace(ws: string, work: WorkspaceWork, email: string): Promise<PushOutcome> {
	const images: AISessionBackupPush[] = []
	const entries: AISessionBackupPush[] = []
	for (const { plan } of work.plans) {
		images.push(...(await imageEntries(plan, email)))
		if (plan.entry) entries.push(plan.entry)
	}
	const failed = new Set<string>()
	for (const requestBody of packRequests(email, images, entries, work.removed)) {
		if (getCurrentUserEmail() !== email) return { failed, disabled: false, abort: true }
		let res
		try {
			res = await AiService.pushAiSessionBackups({ workspace: ws, requestBody })
		} catch (e) {
			const status = statusOf(e)
			// 404: a build without object storage. 403: nothing this token may back up.
			if (status === 404 || status === 403) return { failed, disabled: true, abort: false }
			if (status === 409) return { failed, disabled: false, abort: true }
			if (status !== undefined && status < 500 && status !== 429) {
				console.error('Session backup push refused', e)
				return { failed, disabled: true, abort: false }
			}
			console.warn('Session backup push failed, retrying later', e)
			return undefined
		}
		if (!res.enabled) return { failed, disabled: true, abort: false }
		for (const r of res.results) {
			if (r.error) {
				console.warn(`Session backup of ${r.id} failed: ${r.error}`)
				failed.add(r.id)
			}
		}
	}
	return { failed, disabled: false, abort: false }
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
		await backfillMarks(marks, email)
		if (!hasPending(marks)) return
		const stored = await readStoredSessions(email)
		if (!stored) return
		const byId = new Map(stored.map((s) => [s.id, s]))
		const work = new Map<string, WorkspaceWork>()
		const workFor = (ws: string) => {
			let w = work.get(ws)
			if (!w) work.set(ws, (w = { plans: [], removed: [] }))
			return w
		}
		// Marks with nothing to push, cleared at the end alongside the successful ones.
		const done: { id: string; v: number }[] = []
		const settledSync: MirrorSyncState[] = []

		for (const [id, mark] of Object.entries(marks.dirty)) {
			const session = byId.get(id)
			const sync = await readSync(id, email)
			const plan =
				session && !session.transient ? await planFor(session, mark, sync, email) : undefined
			if (!plan) {
				done.push({ id, v: mark.v })
				continue
			}
			if (plan.removeFrom && wsState.get(plan.removeFrom) !== 'off') {
				workFor(plan.removeFrom).removed.push(id)
			}
			if (wsState.get(plan.workspaceId) === 'off') {
				done.push({ id, v: mark.v })
				continue
			}
			if (!plan.entry && plan.images.length === 0) {
				// Nothing the backup keeps changed; remember what the stores hold now.
				if (JSON.stringify(plan.next) !== JSON.stringify(sync)) settledSync.push(plan.next)
				done.push({ id, v: mark.v })
				continue
			}
			workFor(plan.workspaceId).plans.push({ plan, mark })
		}
		for (const r of marks.removed) {
			const ws = r.ws ?? (await readSync(r.id, email))?.ws
			if (ws && wsState.get(ws) !== 'off') workFor(ws).removed.push(r.id)
		}

		const removedDone = new Set<string>()
		let abort = false
		for (const [ws, w] of work) {
			if (abort) break
			const outcome = await pushWorkspace(ws, w, email)
			if (!outcome) {
				backOff()
				continue
			}
			if (outcome.abort) {
				abort = true
				break
			}
			if (outcome.disabled) {
				disableWorkspace(ws)
				await forgetWorkspaceSync(ws, email)
				for (const { plan, mark } of w.plans) done.push({ id: plan.next.id, v: mark.v })
				for (const id of w.removed) removedDone.add(id)
				continue
			}
			retryMs = RETRY_MIN_MS
			for (const { plan, mark } of w.plans) {
				if (outcome.failed.has(plan.next.id)) continue
				settledSync.push(plan.next)
				done.push({ id: plan.next.id, v: mark.v })
			}
			for (const id of w.removed) if (!outcome.failed.has(id)) removedDone.add(id)
		}
		if (abort || getCurrentUserEmail() !== email) return

		await writeSync(settledSync, email)
		// A removal that also moved the session elsewhere keeps its (new) sync row.
		const movedIds = new Set(settledSync.map((s) => s.id))
		await deleteSync(
			[...removedDone].filter((id) => !movedIds.has(id)),
			email
		)
		// Re-read: marks raised while this flush ran must stay.
		const latest = readPending()
		for (const { id, v } of done) {
			if (latest.dirty[id]?.v === v) delete latest.dirty[id]
		}
		latest.removed = latest.removed.filter((r) => !removedDone.has(r.id))
		writePending(latest)
		if (hasPending(latest) && !abort) scheduleFlush()
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
		if (status === 404 || status === 403) disableWorkspace(ws)
		else console.warn('Could not list session backups', e)
		return
	}
	if (!listing.enabled) {
		disableWorkspace(ws)
		return
	}
	wsState.set(ws, 'on')
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
			disableWorkspace(ws)
			return
		}
		// Ask again for what did not fit, one at a time so each answer is as small as can be.
		for (const id of pulled.deferred) if (!ids.includes(id)) ids.unshift(id)
		const unpacked = pulled.sessions
			.map((b) => unpackBackup(ws, b, updatedAt.get(b.id) ?? Date.now()))
			.filter((u) => u !== undefined)
		if (unpacked.length === 0) continue
		for (const u of unpacked) await importArtifacts(u.artifacts.items, u.artifacts.versions, email)
		await importStoredChats(
			unpacked.flatMap((u) => u.chats),
			unpacked.flatMap((u) => u.images),
			email
		)
		const imported = new Set(
			await importSessions(
				unpacked.map((u) => u.session),
				email
			)
		)
		await writeSync(
			unpacked.filter((u) => imported.has(u.session.id)).map((u) => u.sync),
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
		const marks = readPending()
		if (signal.kind === 'dirty') {
			addDirty(marks, signal.sessionId, signal.chatId)
		} else {
			delete marks.dirty[signal.sessionId]
			marks.removed = marks.removed.filter((r) => r.id !== signal.sessionId)
			marks.removed.push({ id: signal.sessionId, ws: signal.workspaceId })
		}
		writePending(marks)
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
