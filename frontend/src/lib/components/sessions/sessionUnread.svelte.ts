import type { SessionRuntime } from './sessionRuntime.svelte'

// Per-user, per-session "last seen" marker — count of displayMessages the
// last time the user was actually on that session's page. Compared against
// the runtime's current message count to derive an unread badge.
//
// Stored as a single localStorage entry holding Record<sessionId, count>.
// Module-level $state for cross-session reactivity; can't use
// `useLocalStorageValue` here because its internal $effect requires a
// component-initialization context (we run at import time).
const LS_KEY = 'windmill_sessions_last_seen_counts'

function readInitial(): Record<string, number> {
	if (typeof window === 'undefined') return {}
	try {
		const raw = localStorage.getItem(LS_KEY)
		return raw ? JSON.parse(raw) : {}
	} catch {
		return {}
	}
}

const lastSeen = $state<{ val: Record<string, number> }>({ val: readInitial() })

function persist(): void {
	if (typeof window === 'undefined') return
	try {
		localStorage.setItem(LS_KEY, JSON.stringify(lastSeen.val))
	} catch (e) {
		console.error('sessionUnread: localStorage write failed', e)
	}
}

// Mark the session as seen up to `count` messages. No-op when already at
// or past that count (idempotent — call freely from $effects).
export function markSessionSeen(sessionId: string, count: number) {
	const current = lastSeen.val[sessionId] ?? 0
	if (current >= count) return
	lastSeen.val = { ...lastSeen.val, [sessionId]: count }
	persist()
}

// Drop the session entry entirely (used on delete so we don't leak
// stale ids into localStorage indefinitely).
export function forgetSessionSeen(sessionId: string) {
	if (!(sessionId in lastSeen.val)) return
	const next = { ...lastSeen.val }
	delete next[sessionId]
	lastSeen.val = next
	persist()
}

// Number of unread messages for a session. Undefined / unloaded runtime
// returns 0 — until messages are hydrated we don't know what's new.
export function unreadCountFor(sessionId: string, runtime: SessionRuntime | undefined): number {
	if (!runtime) return 0
	const seen = lastSeen.val[sessionId] ?? 0
	const total = runtime.manager.displayMessages.length
	return Math.max(0, total - seen)
}
