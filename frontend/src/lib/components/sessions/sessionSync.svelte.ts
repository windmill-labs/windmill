import { BROWSER } from 'esm-env'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'
import { noteDriverAlive, noteRemoteTurnEnded } from './sessionRunOwner.svelte'

// The channel between tabs open on the same AI session. Everything a session is
// made of — the record list, the chat transcript, the run itself — lives in the
// tab, so two tabs on one session are two independent copies of it. This module
// carries messages between those copies: record writes, and how the driving
// tab's run is going. Who is entitled to drive is sessionRunOwner's question;
// this module only tells it what arrived.
//
// No message carries a transcript. Tabs converge by re-reading the shared
// IndexedDB record once a run ends, so what crosses the channel stays small and
// bounded whatever a turn produced.
//
// The channel is per-user (same email scoping as the IndexedDB stores), so a
// browser shared by two accounts never crosses them.

const CHANNEL_BASE = 'windmill-sessions-sync'

/** How often a driving tab posts its status. An unchanged run still ticks, so
 *  silence is what tells the other tabs the driver is gone; the reaper's
 *  threshold is several times this. */
export const RUN_STATUS_INTERVAL_MS = 1000

/** Carries only the id on purpose. Broadcast delivery order and IndexedDB
 *  commit order are independent, so shipping a copy of the record lets an older
 *  write land on top of a newer one; re-reading converges on what the shared
 *  store actually holds. */
type SessionPutMsg = { kind: 'session-put'; id: string }
type SessionDeleteMsg = { kind: 'session-delete'; id: string }
/** An artifact another tab wrote or removed. Same id-only shape, and for the
 *  same reason: the receiver re-reads, and a read that finds nothing is how a
 *  removal arrives. `sessionId` is here so a receiver can route to the right
 *  store without a database round-trip for artifacts it does not hold. */
type SessionArtifactMsg = { kind: 'session-artifact'; sessionId: string; artifactId: string }
/** Names the chat the driver ended on, which is not necessarily the one it
 *  started on: a "/clear" rotates it mid-turn, and the watcher's re-read has to
 *  follow. Whether the turn landed or errored is not carried, because the
 *  watcher does the same thing either way — re-read the record and stop
 *  showing the run. */
type TurnEndMsg = { kind: 'turn-end'; sessionId: string; chatId: string }
/**
 * How the driving tab's run is going. Deliberately carries no transcript: a
 * watching tab keeps showing its own, and drives the same loading indicator a
 * local turn shows from these fields, then re-reads the record once `turn-end`
 * says the run is over.
 *
 * Every field here is a scalar of fixed size. That is the property to preserve:
 * the transcript this replaced carried tool results and job logs, which are
 * bounded by nothing, and re-broadcasting them several times a second was a
 * responsiveness bug that recurred once per field anyone forgot to strip.
 */
type RunStatusMsg = {
	kind: 'run-status'
	sessionId: string
	/** Identifies the turn. A watcher echoes it back on a control message so one
	 *  delayed past the turn's end cannot act on the turn that followed. */
	runId: string
	loading: boolean
	compacting: boolean
	/** Parked on a question that only the driving tab can render. The watcher
	 *  says where to answer it rather than implying work is in progress. */
	blockedOnUser: boolean
	loadingLabel: string | undefined
	/** The driver's plan-mode posture. The only autonomy state worth carrying:
	 *  every other one is a stored preference each tab keeps its own copy of,
	 *  while plan mode is never persisted and so exists nowhere but the driving
	 *  tab's memory. */
	planModeActive: boolean
}
/** A Stop pressed in a tab that is only watching the run, named for the turn it
 *  was pressed during: delivery is asynchronous, so it can arrive after that turn
 *  ended and a later one began, and cancelling is not undoable. The sends a turn
 *  spawns itself share its id — they are one run, and Stop should take the lot. */
type CancelRequestMsg = { kind: 'cancel-request'; sessionId: string; runId: string }
type SyncMsg =
	| SessionPutMsg
	| SessionDeleteMsg
	| SessionArtifactMsg
	| TurnEndMsg
	| RunStatusMsg
	| CancelRequestMsg

type Handlers = {
	onSessionPut: (id: string) => void
	onSessionDelete: (id: string) => void
	onSessionArtifact: (sessionId: string, artifactId: string) => void
	onTurnEnd: (sessionId: string, chatId: string) => void
	onRunStatus: (msg: RunStatusMsg) => void
	onCancelRequest: (sessionId: string, runId: string) => void
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
		case 'session-artifact':
			emit('onSessionArtifact', msg.sessionId, msg.artifactId)
			break
		case 'turn-end':
			noteRemoteTurnEnded(msg.sessionId)
			emit('onTurnEnd', msg.sessionId, msg.chatId)
			break
		case 'run-status':
			noteDriverAlive(msg.sessionId, msg.planModeActive, msg.runId)
			emit('onRunStatus', msg)
			break
		case 'cancel-request':
			emit('onCancelRequest', msg.sessionId, msg.runId)
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

export function broadcastSessionArtifact(sessionId: string, artifactId: string): void {
	post({ kind: 'session-artifact', sessionId, artifactId })
}

// ---------------------------------------------------------------------------
// Run status
// ---------------------------------------------------------------------------

export function broadcastTurnEnd(sessionId: string, chatId: string): void {
	post({ kind: 'turn-end', sessionId, chatId })
}

export function requestCancel(sessionId: string, runId: string): void {
	post({ kind: 'cancel-request', sessionId, runId })
}

export type RunStatus = Omit<RunStatusMsg, 'kind'>

/** Say how the run this tab is driving is going. */
export function broadcastRunStatus(status: RunStatus): void {
	post({ kind: 'run-status', ...status })
}

export type { RunStatusMsg }
