// The one-way channel from the IndexedDB write funnels (session records, chat history,
// artifacts) to the session backup. Import-free on purpose: the stores it is called from
// must not depend on the backup module, which depends on all of them.

// `email` names the user whose store the write landed in (from the store's scoped name):
// the current user may have changed while the write was pending, and the mark belongs to
// the store's user, not to whoever is logged in when it completes.
export type MirrorSignal =
	| { kind: 'dirty'; sessionId: string; chatId?: string; email?: string }
	| { kind: 'removed'; sessionId: string; workspaceId?: string; email?: string }

let handler: ((signal: MirrorSignal) => void) | undefined
// Signals raised before the backup module registered, replayed to it on registration.
let buffered: MirrorSignal[] = []

function emit(signal: MirrorSignal): void {
	if (handler) handler(signal)
	else buffered.push(signal)
}

/** A durable local write landed for this session (and, when known, this chat) in the
 * store of `email`. */
export function markSessionDirty(sessionId: string, chatId?: string, email?: string): void {
	emit({ kind: 'dirty', sessionId, chatId, email })
}

/** The user deleted this session; its backup goes with it. */
export function markSessionRemoved(sessionId: string, workspaceId?: string, email?: string): void {
	emit({ kind: 'removed', sessionId, workspaceId, email })
}

export function onMirrorSignal(fn: (signal: MirrorSignal) => void): void {
	handler = fn
	const replay = buffered
	buffered = []
	for (const signal of replay) fn(signal)
}

export function __resetMirrorSignalForTesting(): void {
	handler = undefined
	buffered = []
}
