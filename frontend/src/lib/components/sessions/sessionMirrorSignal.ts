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

/** The Web Lock one tab of the user holds while it reads or writes the stores wholesale: the
 * backup's flush and restore, and the retention sweep, which must not interleave with either
 * (a flush planning a session half deleted would push the deletions to the backup). */
export function sessionsLockName(email: string): string {
	return `wm-ai-sessions-mirror::${email}`
}

export function onMirrorSignal(fn: (signal: MirrorSignal) => void): void {
	handler = fn
	const replay = buffered
	buffered = []
	for (const signal of replay) fn(signal)
}

let sweptHandler: ((sessionId: string, email: string) => Promise<void>) | undefined

/** The retention sweep deleted this session's local copy in the store of `email`: what the
 * backup keeps of it in this browser goes too. Awaited under the sweep's tab lock. */
export async function sessionSwept(sessionId: string, email: string): Promise<void> {
	try {
		await sweptHandler?.(sessionId, email)
	} catch (e) {
		console.error('Could not forget the backup state of a swept session', e)
	}
}

export function onSessionSwept(fn: (sessionId: string, email: string) => Promise<void>): void {
	sweptHandler = fn
}

export function __resetMirrorSignalForTesting(): void {
	handler = undefined
	sweptHandler = undefined
	buffered = []
}
