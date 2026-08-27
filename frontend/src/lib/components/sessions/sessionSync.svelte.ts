import { BROWSER } from 'esm-env'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import { noteDriverAlive, noteRemoteTurnEnded } from './sessionRunOwner.svelte'

// The channel between tabs open on the same AI session. Everything a session is
// made of — the record list, the chat transcript, the run itself — lives in the
// tab, so two tabs on one session are two independent copies of it. This module
// carries messages between those copies: record writes, and the driving tab's
// live transcript. Who is entitled to drive is sessionRunOwner's question; this
// module only tells it what arrived.
//
// The channel is per-user (same email scoping as the IndexedDB stores), so a
// browser shared by two accounts never crosses them.

const CHANNEL_BASE = 'windmill-sessions-sync'

/** Mirror ticks are throttled to this while a turn streams. Also the heartbeat
 *  interval: an unchanged run still ticks, so silence means the driver is gone. */
export const MIRROR_THROTTLE_MS = 250

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
	/** The driver's plan-mode posture. The only autonomy state worth carrying:
	 *  every other one is a stored preference each tab keeps its own copy of,
	 *  while plan mode is never persisted and so exists nowhere but the driving
	 *  tab's memory. */
	planModeActive: boolean
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
	| SessionArtifactMsg
	| TurnEndMsg
	| MirrorMsg
	| ResyncRequestMsg
	| CancelRequestMsg
	| ToolConfirmationMsg
	| QuestionAnswerMsg

type Handlers = {
	onSessionPut: (id: string) => void
	onSessionDelete: (id: string) => void
	onSessionArtifact: (sessionId: string, artifactId: string) => void
	onTurnEnd: (sessionId: string, chatId: string) => void
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
		case 'session-artifact':
			emit('onSessionArtifact', msg.sessionId, msg.artifactId)
			break
		case 'turn-end':
			noteRemoteTurnEnded(msg.sessionId)
			emit('onTurnEnd', msg.sessionId, msg.chatId)
			break
		case 'mirror':
			noteDriverAlive(msg.sessionId, msg.planModeActive)
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

export function broadcastSessionArtifact(sessionId: string, artifactId: string): void {
	post({ kind: 'session-artifact', sessionId, artifactId })
}

// ---------------------------------------------------------------------------
// Live mirroring
// ---------------------------------------------------------------------------

export function broadcastTurnEnd(sessionId: string, chatId: string): void {
	post({ kind: 'turn-end', sessionId, chatId })
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

/** Send the driver's current view of a run. */
export function broadcastMirror(snap: MirrorSnapshot): void {
	post({ kind: 'mirror', ...snap })
}

export type { MirrorMsg }
