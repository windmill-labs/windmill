// The live session runtimes, and what the sidebar reads of a session without one. Apart from
// `./sessionRuntime.svelte`, which builds runtimes and reaches the whole chat engine: the
// sidebar lists sessions on every page, and only opening one needs the engine.
import { SvelteMap } from 'svelte/reactivity'
import type {
	ChatJob,
	DisplayMessage,
	PendingUserAction
} from '$lib/components/copilot/chat/shared'
import { readStoredChat } from '$lib/components/copilot/chat/HistoryManager.svelte'
import { getCurrentUserEmail } from '$lib/userScopedStorage'
import { onRemoteTurnEnd } from './sessionSync.svelte'
import {
	adoptStoredSessionWorkspace,
	deleteSession as deleteSessionState,
	noteRemoteStateChange,
	sessionState,
	setDraftsDiscardedHandler,
	setSendUnderWayCheck,
	setSessionChatId,
	type Session
} from './sessionState.svelte'
import type { SessionRuntime } from './sessionRuntime.svelte'

export const runtimes = new SvelteMap<string, SessionRuntime>()
export const runtimeRoots = new Map<string, () => void>()

// Only ever asked of a runtime, and every runtime is built by `./sessionRuntime.svelte`, which
// sets this when it loads.
let pendingActionOf: (messages: DisplayMessage[]) => PendingUserAction | undefined = () => undefined
export function setPendingUserActionResolver(
	fn: (messages: DisplayMessage[]) => PendingUserAction | undefined
): void {
	pendingActionOf = fn
}

export function disposeRuntime(sessionId: string) {
	const runtime = runtimes.get(sessionId)
	if (!runtime) return
	runtime.manager.cancel('runtime disposed')
	runtime.manager.historyManager.close()
	runtimes.delete(sessionId)
	runtimeRoots.get(sessionId)?.()
	runtimeRoots.delete(sessionId)
	// The sidebar reads the stored chat again rather than trust a read that
	// predates everything this runtime saw.
	peeks.delete(sessionId)
}

export function listRuntimes(): SessionRuntime[] {
	return Array.from(runtimes.values())
}

export function getRuntime(sessionId: string): SessionRuntime | undefined {
	return runtimes.get(sessionId)
}

// ---------------------------------------------------------------------------
// Cross-tab catch-up
// ---------------------------------------------------------------------------

// Chained per session so two turn-ends close together (a turn plus its queued
// follow-up) re-read sequentially: the later read starts after the earlier
// one's loadPastChat, so the newest record is what ends up on screen.
const catchUps = new Map<string, Promise<void>>()

onRemoteTurnEnd((sessionId, chatId) => {
	noteRemoteStateChange(sessionId)
	const next = (catchUps.get(sessionId) ?? Promise.resolve())
		.then(() => applyRemoteTurnEnd(sessionId, chatId))
		.catch((e) => console.error('Failed to catch up on a turn from another tab', e))
	catchUps.set(sessionId, next)
	void next.finally(() => {
		if (catchUps.get(sessionId) === next) catchUps.delete(sessionId)
	})
	// Awaited by the caller: the composer unlock rides on this settling.
	return next
})

async function applyRemoteTurnEnd(sessionId: string, chatId: string): Promise<void> {
	// The other tab may have moved the session to a fork (it brackets the move like a turn).
	const moved = await adoptStoredSessionWorkspace(sessionId)
	const runtime = runtimes.get(sessionId)
	if (moved) runtime?.manager.rebuildGlobalSystemMessage()
	if (!runtime) {
		// The driving tab may have rotated to a new chat; follow it, or the next
		// read and a later runtime would open the old one.
		setSessionChatId(sessionId, chatId)
		refreshSessionChatPeek(sessionId)
		return
	}
	const m = runtime.manager
	// Two transient states get a short retry rather than a skip, because the
	// composer unlocks when this promise settles and a skip would unlock it on
	// stale history: a send of this tab's own still in preflight (it may yet be
	// refused, leaving no turn to converge on), and a store that failed to
	// open. A turn actually running here owns the transcript instead — its own
	// end converges — and the pruner caps the whole hold at STALE_MS anyway.
	for (let attempt = 0; ; attempt++) {
		if (m.loading) return
		if (!m.sendInFlight) {
			const res = await m.historyManager.reloadChat(chatId)
			if (res === 'missing') return
			if (res === 'loaded') break
		}
		if (attempt >= 7) return
		await new Promise((r) => setTimeout(r, 500))
		if (runtimes.get(sessionId) !== runtime) return
	}
	// Disposed (session deleted, teardown) while the read was in flight.
	if (runtimes.get(sessionId) !== runtime) return
	// Adopts the driver's chat unconditionally, current view included: watching
	// a session means following where its activity is, and it is also how tabs
	// converge after an unsynced /clear rotation. A watcher browsing an older
	// conversation is pulled along — deliberate, and the price of not syncing
	// rotation as its own message.
	//
	// preserveQueue: this reload is a catch-up, not a conversation switch — a
	// draft queued here (a refused send's kept message, a failed turn's card)
	// is unsent user input the re-read must not destroy.
	await m.loadPastChat(chatId, { preserveQueue: true })
	// loadPastChat's own artifact sync no-ops for an unchanged session id, so
	// artifacts the driver wrote during the turn need this forced re-read.
	await m.artifacts.resyncFromStore()
}

export type SessionChatStatus =
	| 'idle'
	| 'streaming'
	// The assistant's turn ended and the user has yet to reply — passive, unlike
	// 'awaiting-answer'/'needs-confirmation', where a running loop is blocked.
	| 'awaiting-user'
	| 'awaiting-answer'
	| 'needs-confirmation'
	| 'draft'
	| 'error'

// Full session teardown: dispose the runtime and remove from sessionState in one
// call. Callers (sidebar / header dropdowns) just invoke this; navigation away
// from a deleted active session is the caller's responsibility.
export function removeSession(sessionId: string): void {
	disposeRuntime(sessionId)
	deleteSessionState(sessionId)
}

setDraftsDiscardedHandler((ids) => ids.forEach(disposeRuntime))
setSendUnderWayCheck((id) => {
	const m = runtimes.get(id)?.manager
	return !!m && (m.loading || m.sendInFlight || m.sendPending)
})

export function getSessionChatStatus(runtime: SessionRuntime): SessionChatStatus {
	const m = runtime.manager
	const last = m.displayMessages[m.displayMessages.length - 1]
	// A loop parked on the user still reports `loading`, so these must be tested
	// before `streaming` — otherwise "answer me" renders as "the AI is typing"
	// and a session that needs the user looks like one that doesn't.
	const pending = pendingActionOf(m.displayMessages)
	if (pending === 'question') return 'awaiting-answer'
	if (pending === 'confirmation') return 'needs-confirmation'
	if (m.loading) return 'streaming'
	if (m.instructions.trim().length > 0) return 'draft'
	if (last?.role === 'user' && last.error) return 'error'
	if (last && (last.role === 'assistant' || last.role === 'tool')) return 'awaiting-user'
	return 'idle'
}

// ---------------------------------------------------------------------------
// Sessions without a runtime
// ---------------------------------------------------------------------------

// A runtime costs a chat manager and its loaded transcript, and lives until its
// session is deleted, so the sidebar does not create one per listed session. It
// reads the session's one stored chat instead, once. Without a runtime nothing
// runs in this tab, so only two facts of that chat can matter: its length (for
// the unread count) and whether its last message failed. Every other status is
// a live one.
export interface SessionChatPeek {
	status: 'error' | 'idle'
	messageCount: number
}

const peeks = new SvelteMap<string, SessionChatPeek>()
const peeksInFlight = new Set<string>()
const peeksPendingRefresh = new Set<string>()

// Not gated on `detached`: a chat saved mid-wait stores its job undetached, and
// loadPastChat resumes polling every unfinished job either way.
function isLiveJob(j: ChatJob): boolean {
	return (
		j.status === 'queued' ||
		j.status === 'running' ||
		j.status === 'suspended' ||
		j.status === 'scheduled'
	)
}

function sessionById(sessionId: string): Session | undefined {
	return sessionState.sessions.find((s) => s.id === sessionId)
}

function refreshSessionChatPeek(sessionId: string): void {
	peeks.delete(sessionId)
	if (peeksInFlight.has(sessionId)) {
		peeksPendingRefresh.add(sessionId)
		return
	}
	const session = sessionById(sessionId)
	if (session) void ensureSessionChatPeek(session)
}

export function getSessionChatPeek(sessionId: string): SessionChatPeek | undefined {
	return peeks.get(sessionId)
}

/** Read the stored chat of a session that has no runtime. A chat with a detached
 *  job still running gets its runtime instead: only a runtime polls the job and
 *  resumes the conversation when it finishes. */
export async function ensureSessionChatPeek(session: Session): Promise<void> {
	const id = session.id
	const chatId = session.chatId
	if (runtimes.has(id) || peeks.has(id) || peeksInFlight.has(id) || !chatId) return
	const email = getCurrentUserEmail()
	if (!email) return
	peeksInFlight.add(id)
	try {
		const chat = await readStoredChat(chatId, email)
		if (sessionById(id)?.chatId !== chatId) {
			refreshSessionChatPeek(id)
			return
		}
		if (!chat || runtimes.has(id)) return
		if (chat.backgroundJobs?.some(isLiveJob)) {
			const { getOrCreateRuntime } = await import('./sessionRuntime.svelte')
			getOrCreateRuntime(session)
			return
		}
		const last = chat.displayMessages[chat.displayMessages.length - 1]
		peeks.set(id, {
			status: last?.role === 'user' && last.error ? 'error' : 'idle',
			messageCount: chat.displayMessages.length
		})
	} catch (e) {
		console.error('Failed to read session chat', e)
	} finally {
		peeksInFlight.delete(id)
		if (peeksPendingRefresh.delete(id)) {
			refreshSessionChatPeek(id)
		}
	}
}
