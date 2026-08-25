import { BROWSER } from 'esm-env'
import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'

// Cross-tab coordination for AI sessions. Everything a session is made of —
// the record list, the chat transcript, the run itself — lives in the tab, so
// two tabs on the same session are two independent copies of it. This module
// is the one channel between them: it mirrors record writes, elects a single
// driving tab per run, and streams the driver's live transcript to the others.
//
// Why a run needs an owner at all: two tabs sending on one session append to
// the same chat id, and whichever saveChat lands last silently drops the other's
// turn — after its tool calls have already run against the workspace.
//
// The channel is per-user (same email scoping as the IndexedDB stores), so a
// browser shared by two accounts never crosses them.

const CHANNEL_BASE = 'windmill-sessions-sync'

/** Web Locks is secure-context only; BroadcastChannel is not. Without it there
 *  is no way to retire a driver that closed mid-turn, and a watcher would sit on
 *  "generating" forever with a Stop that reaches nobody — worse than not
 *  mirroring at all. So run mirroring rides on this and record sync, which needs
 *  no ownership, does not. */
export const RUN_OWNERSHIP_AVAILABLE = BROWSER && !!globalThis.navigator?.locks?.query

/** Tail length of the transcript sent on each mirror tick. In-place edits to
 *  already-rendered messages (tool cards settling) land within a few messages
 *  of the end, so a short tail carries them while keeping the payload bounded
 *  on long conversations. Anything older is corrected by the IndexedDB re-read
 *  at turn end. */
const MIRROR_TAIL = 10

/** Mirror ticks are throttled to this while a turn streams. Also the heartbeat
 *  interval: an unchanged run still ticks, so silence means the driver is gone. */
export const MIRROR_THROTTLE_MS = 250

/** A driver with no mirror for this long is presumed dead, pending the lock
 *  query that confirms it. Generous next to the heartbeat: a busy tab can be
 *  starved of frames for a while without actually having gone away. */
const MIRROR_SILENCE_MS = 10_000

/** Carries only the id on purpose. Broadcast delivery order and IndexedDB
 *  commit order are independent, so shipping a copy of the record lets an older
 *  write land on top of a newer one; re-reading converges on what the shared
 *  store actually holds. */
type SessionPutMsg = { kind: 'session-put'; id: string }
type SessionDeleteMsg = { kind: 'session-delete'; id: string }
/** `committed` distinguishes a turn that landed from one that errored, was
 *  rolled back, or belonged to a tab that vanished. Watchers auto-send what the
 *  user queued only on the first, matching the rule a turn follows locally. */
type TurnEndMsg = { kind: 'turn-end'; sessionId: string; chatId: string; committed: boolean }
type MirrorMsg = {
	kind: 'mirror'
	sessionId: string
	chatId: string
	/** Index the tail starts at; the receiver keeps its own prefix below it, and
	 *  asks for a full snapshot when it has no prefix that reaches this far. */
	baseIndex: number
	tail: DisplayMessage[]
	/** The driver's whole transcript length. A watcher holding more than this has
	 *  a prefix the driver no longer has, so its splice would invent a transcript
	 *  that never existed; it resyncs instead. */
	total: number
	loading: boolean
	currentReply: string
	currentReasoning: string
	currentReasoningActive: boolean
	loadingLabel: string | undefined
	compacting: boolean
}
/** Sent by a tab whose local prefix can't host the tail it just received (it
 *  joined mid-run, or is on a different chat). The driver answers with a full
 *  snapshot. */
type ResyncRequestMsg = { kind: 'resync-request'; sessionId: string }
/** A Stop pressed in a tab that is only watching the run. */
type CancelRequestMsg = { kind: 'cancel-request'; sessionId: string }
/** A run blocked on the user is unblocked from whichever tab the user is in;
 *  the resolver waiting on the answer only exists in the driving tab. */
type ToolConfirmationMsg = {
	kind: 'tool-confirmation'
	sessionId: string
	toolId: string
	confirmed: boolean
}
type QuestionAnswerMsg = {
	kind: 'question-answer'
	sessionId: string
	toolId: string
	choices: string[]
}

type SyncMsg =
	| SessionPutMsg
	| SessionDeleteMsg
	| TurnEndMsg
	| MirrorMsg
	| ResyncRequestMsg
	| CancelRequestMsg
	| ToolConfirmationMsg
	| QuestionAnswerMsg

type Handlers = {
	onSessionPut: (id: string) => void
	onSessionDelete: (id: string) => void
	onTurnEnd: (sessionId: string, chatId: string, committed: boolean) => void
	onMirror: (msg: MirrorMsg) => void
	onResyncRequest: (sessionId: string) => void
	onCancelRequest: (sessionId: string) => void
	onToolConfirmation: (sessionId: string, toolId: string, confirmed: boolean) => void
	onQuestionAnswer: (sessionId: string, toolId: string, choices: string[]) => void
}

const subscribers: Partial<Handlers>[] = []

/** Registered by sessionState (records) and sessionRuntime (runs) at module
 *  load. Split so this module stays free of both their imports — it would
 *  otherwise sit in an import cycle with each. Several subscribers can claim
 *  the same event: a remote delete has to reach the record list AND tear down
 *  the runtime, and neither module can call the other. */
export function registerSyncHandlers(h: Partial<Handlers>): void {
	subscribers.push(h)
}

function emit<K extends keyof Handlers>(event: K, ...args: Parameters<Handlers[K]>): void {
	for (const sub of subscribers) {
		const fn = sub[event] as ((...a: Parameters<Handlers[K]>) => void) | undefined
		if (!fn) continue
		try {
			fn(...args)
		} catch (e) {
			// One bad subscriber must not strand the others mid-broadcast.
			console.error(`sessionSync: ${event} handler failed`, e)
		}
	}
}

let channel: BroadcastChannel | undefined
let channelName: string | undefined

function openChannel(): void {
	const name = scopedKey(CHANNEL_BASE)
	if (name === channelName) return
	channel?.close()
	channel = undefined
	channelName = name
	if (!name) return
	try {
		const ch = new BroadcastChannel(name)
		ch.onmessage = (ev: MessageEvent<SyncMsg>) => receive(ev.data)
		channel = ch
	} catch (e) {
		// No BroadcastChannel (or blocked): every tab simply stays independent,
		// which is the pre-sync behavior rather than a broken one.
		console.error('sessionSync: could not open channel', e)
	}
}

if (BROWSER) {
	onUserChange(() => {
		// A user switch rescopes the channel name, so the previous identity's
		// channel is closed before the next one opens.
		openChannel()
	})
}

function receive(msg: SyncMsg): void {
	switch (msg.kind) {
		case 'session-put':
			emit('onSessionPut', msg.id)
			break
		case 'session-delete':
			emit('onSessionDelete', msg.id)
			break
		case 'turn-end':
			remoteDriven.delete(msg.sessionId)
			emit('onTurnEnd', msg.sessionId, msg.chatId, msg.committed)
			break
		case 'mirror':
			noteDriverAlive(msg.sessionId)
			emit('onMirror', msg)
			break
		case 'resync-request':
			emit('onResyncRequest', msg.sessionId)
			break
		case 'cancel-request':
			emit('onCancelRequest', msg.sessionId)
			break
		case 'tool-confirmation':
			emit('onToolConfirmation', msg.sessionId, msg.toolId, msg.confirmed)
			break
		case 'question-answer':
			emit('onQuestionAnswer', msg.sessionId, msg.toolId, msg.choices)
			break
	}
}

function post(msg: SyncMsg): void {
	if (!channel) return
	try {
		channel.postMessage(msg)
	} catch (e) {
		// A non-cloneable payload must never take the turn down with it.
		console.error('sessionSync: could not post message', e)
	}
}

// ---------------------------------------------------------------------------
// Record sync
// ---------------------------------------------------------------------------

export function broadcastSessionPut(id: string): void {
	post({ kind: 'session-put', id })
}

export function broadcastSessionDelete(id: string): void {
	post({ kind: 'session-delete', id })
}

// ---------------------------------------------------------------------------
// Ownership
// ---------------------------------------------------------------------------

/** Sessions currently being driven by another tab, with the time of the last
 *  sign of life. Reactive so a tab that starts or stops driving re-renders the
 *  gates that read it. */
const remoteDriven = new SvelteMap<string, { lastAt: number }>()

function noteDriverAlive(sessionId: string): void {
	remoteDriven.set(sessionId, { lastAt: Date.now() })
	ensureReaper()
}

// Runs only while some session is being driven elsewhere, and stops itself once
// none is — a browser with a single tab open never arms it at all.
let reaperTimer: ReturnType<typeof setInterval> | undefined

function ensureReaper(): void {
	if (reaperTimer) return
	reaperTimer = setInterval(() => {
		if (remoteDriven.size === 0) {
			clearInterval(reaperTimer)
			reaperTimer = undefined
			return
		}
		void reapDeadDrivers()
	}, MIRROR_SILENCE_MS)
}

export function isRemotelyDriven(sessionId: string): boolean {
	return remoteDriven.has(sessionId)
}

/** Sessions this tab is currently driving. Reactive so the gates that read it
 *  re-render when a run starts or ends. */
const locallyDriven = new SvelteSet<string>()

export function isLocallyDriven(sessionId: string): boolean {
	return locallyDriven.has(sessionId)
}

function lockName(sessionId: string): string {
	return `wm-session-run:${sessionId}`
}

/** Run `body` as the session's sole driver, or return 'busy' without running it
 *  when another tab already holds the run.
 *
 *  Web Locks is what makes this safe across a crash: the lock is held by the tab,
 *  not by a record someone has to clean up, so a driver that dies mid-turn
 *  releases it and the next send succeeds. */
export async function withSessionRunLock<T>(
	sessionId: string,
	body: () => Promise<T>
): Promise<T | 'busy'> {
	// No ownership to take: the run proceeds unguarded, and the caller mirrors
	// nothing (see RUN_OWNERSHIP_AVAILABLE), leaving tabs as independent as they
	// were before any of this.
	if (!RUN_OWNERSHIP_AVAILABLE) return body()
	return (await navigator.locks.request(
		lockName(sessionId),
		{ mode: 'exclusive', ifAvailable: true },
		async (lock) => {
			// `ifAvailable` hands back a null lock instead of queueing when another
			// tab holds it, which is exactly the "refuse, don't stack up turns"
			// behavior we want.
			if (!lock) return 'busy' as const
			locallyDriven.add(sessionId)
			try {
				return await body()
			} finally {
				locallyDriven.delete(sessionId)
			}
		}
	)) as T | 'busy'
}

/** Whether any tab currently holds the run lock for this session. Used to
 *  settle a driver that stopped mirroring: a released lock proves the tab is
 *  gone, where silence alone only suggests it. */
async function runLockHeld(sessionId: string): Promise<boolean> {
	try {
		const state = await navigator.locks.query()
		const name = lockName(sessionId)
		return !!state.held?.some((l) => l.name === name)
	} catch {
		return true
	}
}

/** Drop drivers that have gone silent and whose lock is no longer held, so a
 *  closed tab can't leave a session showing "generating" forever. */
async function reapDeadDrivers(): Promise<void> {
	const now = Date.now()
	const stale = [...remoteDriven.entries()]
		.filter(([, v]) => now - v.lastAt > MIRROR_SILENCE_MS)
		.map(([id]) => id)
	for (const id of stale) {
		if (!(await runLockHeld(id))) {
			remoteDriven.delete(id)
			// A driver that disappeared mid-turn committed nothing.
			emit('onTurnEnd', id, '', false)
		}
	}
}

// ---------------------------------------------------------------------------
// Live mirroring
// ---------------------------------------------------------------------------

export function broadcastTurnEnd(sessionId: string, chatId: string, committed: boolean): void {
	post({ kind: 'turn-end', sessionId, chatId, committed })
}

export function requestResync(sessionId: string): void {
	post({ kind: 'resync-request', sessionId })
}

export function requestCancel(sessionId: string): void {
	post({ kind: 'cancel-request', sessionId })
}

export function sendToolConfirmation(sessionId: string, toolId: string, confirmed: boolean): void {
	post({ kind: 'tool-confirmation', sessionId, toolId, confirmed })
}

export function sendQuestionAnswer(sessionId: string, toolId: string, choices: string[]): void {
	post({ kind: 'question-answer', sessionId, toolId, choices })
}

export type MirrorSnapshot = Omit<MirrorMsg, 'kind'>

/** Where the tail a frame carries should start. Callers slice — and only then
 *  clone — so a heartbeat on a long conversation never copies the messages it
 *  is not going to send; a transcript holding pasted files or image data URLs
 *  makes that difference megabytes per tick. `full` sends everything, which is
 *  what answers a resync request. */
export function mirrorBaseIndex(total: number, full = false): number {
	return full ? 0 : Math.max(0, total - MIRROR_TAIL)
}

/** Send the driver's current view of a run. */
export function broadcastMirror(snap: MirrorSnapshot): void {
	post({ kind: 'mirror', ...snap })
}

export type { MirrorMsg }
