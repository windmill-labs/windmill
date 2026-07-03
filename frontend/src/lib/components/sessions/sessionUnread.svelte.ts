import type { SessionRuntime } from './sessionRuntime.svelte'
import { sessionState, putSession } from './sessionState.svelte'

// Per-session "last seen" watermark — the displayMessages count the last time
// the user was on that session's page, compared against the runtime's current
// message count to derive the unread badge. The watermark lives on the session
// record (Session.lastSeenCount), so it is user-scoped, migrated, and deleted
// together with the session itself — no separate store, no cascade cleanup.

// Mark the session as seen up to `count` messages. No-op when already at or
// past that count (idempotent — call freely from $effects).
export function markSessionSeen(sessionId: string, count: number) {
	const s = sessionState.sessions.find((x) => x.id === sessionId)
	if (!s) return
	if ((s.lastSeenCount ?? 0) >= count) return
	s.lastSeenCount = count
	void putSession(s)
}

// Number of unread messages for a session. Undefined / unloaded runtime
// returns 0 — until messages are hydrated we don't know what's new.
export function unreadCountFor(sessionId: string, runtime: SessionRuntime | undefined): number {
	if (!runtime) return 0
	const seen = sessionState.sessions.find((x) => x.id === sessionId)?.lastSeenCount ?? 0
	const total = runtime.manager.displayMessages.length
	return Math.max(0, total - seen)
}
