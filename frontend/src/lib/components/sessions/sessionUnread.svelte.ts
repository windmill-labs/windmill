import type { SessionRuntime } from './sessionRuntime.svelte'
import { scopedKey, onUserChange, migrateLegacyLocalStorage } from '$lib/userScopedStorage'

// Per-user, per-session "last seen" marker — count of displayMessages the
// last time the user was actually on that session's page. Compared against
// the runtime's current message count to derive an unread badge.
//
// Stored as a single localStorage entry holding Record<sessionId, count>,
// namespaced by the logged-in user's email (scopedKey) so unread badges are
// never shared across users on a shared browser. The bare key is also the
// legacy (pre-namespacing) key, claimed once on first login.
const LS_KEY = 'windmill_sessions_last_seen_counts'

function readScoped(): Record<string, number> {
	const key = scopedKey(LS_KEY)
	if (typeof window === 'undefined' || !key) return {}
	try {
		const raw = localStorage.getItem(key)
		return raw ? JSON.parse(raw) : {}
	} catch {
		return {}
	}
}

// Starts empty: hydrated from the user-scoped key by the onUserChange handler
// below once the logged-in email resolves (async, after layout load).
const lastSeen = $state<{ val: Record<string, number> }>({ val: {} })

onUserChange((email, prevEmail) => {
	if (typeof window === 'undefined') return
	const key = scopedKey(LS_KEY)
	if (!key) {
		if (prevEmail !== undefined) lastSeen.val = {}
		return
	}
	migrateLegacyLocalStorage(LS_KEY, key)
	lastSeen.val = readScoped()
})

function persist(): void {
	const key = scopedKey(LS_KEY)
	if (typeof window === 'undefined' || !key) return
	try {
		localStorage.setItem(key, JSON.stringify(lastSeen.val))
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
