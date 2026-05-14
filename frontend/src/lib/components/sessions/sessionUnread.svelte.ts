import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
import type { SessionRuntime } from './sessionRuntime.svelte'

// Per-user, per-session "last seen" marker — count of displayMessages the
// last time the user was actually on that session's page. Compared against
// the runtime's current message count to derive an unread badge.
//
// Stored as a single localStorage entry holding Record<sessionId, count>
// to avoid scattering keys across the namespace and to make
// useLocalStorageValue's reactivity cover all sessions at once.
const lastSeenStore = useLocalStorageValue<Record<string, number>>(
	'windmill_sessions_last_seen_counts',
	{}
)

// Mark the session as seen up to `count` messages. No-op when already at
// or past that count (idempotent — call freely from $effects).
export function markSessionSeen(sessionId: string, count: number) {
	const current = lastSeenStore.val[sessionId] ?? 0
	if (current >= count) return
	lastSeenStore.val = { ...lastSeenStore.val, [sessionId]: count }
}

// Drop the session entry entirely (used on delete so we don't leak
// stale ids into localStorage indefinitely).
export function forgetSessionSeen(sessionId: string) {
	if (!(sessionId in lastSeenStore.val)) return
	const next = { ...lastSeenStore.val }
	delete next[sessionId]
	lastSeenStore.val = next
}

// Number of unread messages for a session. Undefined / unloaded runtime
// returns 0 — until messages are hydrated we don't know what's new.
export function unreadCountFor(sessionId: string, runtime: SessionRuntime | undefined): number {
	if (!runtime) return 0
	const seen = lastSeenStore.val[sessionId] ?? 0
	const total = runtime.manager.displayMessages.length
	return Math.max(0, total - seen)
}
