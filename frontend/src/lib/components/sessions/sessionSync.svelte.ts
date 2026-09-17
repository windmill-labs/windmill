import { BROWSER } from 'esm-env'
import { SvelteMap } from 'svelte/reactivity'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'
import { randomUUID } from '$lib/utils/uuid'

// Cross-tab awareness for AI sessions. Invariant: no message carries state —
// a heartbeat is presence, turn-end triggers an idempotent re-read of the
// shared IndexedDB record — so tabs converge on the store, never on delivery
// order. The lock is advisory (a broadcast-latency race stays last-writer-
// wins, as with no channel), and the channel is per-user like the stores.

const CHANNEL_BASE = 'windmill-sessions-sync'

/** Silence past STALE_MS unlocks watchers a dead driver would strand. The
 *  window sits above the 1/min floor browsers throttle a hidden tab's timers
 *  to — and a hidden driver is the normal case here. Only an uncleanly killed
 *  tab waits it out; a closed one says goodbye via the pagehide farewell. */
const HEARTBEAT_MS = 3_000
const STALE_MS = 90_000
const PRUNE_MS = 2_000

// `from` identifies the driving tab: two drivers racing on one session (the
// documented advisory race) hold separate slots, so one's turn-end can never
// unlock a watcher the other still holds.
export type SyncMsg =
	| { kind: 'run-heartbeat'; sessionId: string; from: string }
	| { kind: 'turn-end'; sessionId: string; chatId: string; from: string }

/** This tab's identity on the channel (a tab never receives its own posts). */
const TAB_ID = randomUUID()

// One slot per (session, driving tab), keyed with a separator no UUID contains.
// The value is a fresh object per message: turn-end's deferred cleanup asks
// "is this slot still mine?" by identity — a timestamp can't, since a same-
// millisecond follow-up heartbeat would compare equal and be deleted.
const remoteRuns = new SvelteMap<string, { at: number }>()

function runKey(sessionId: string, from: string): string {
	return sessionId + ':' + from
}

export function runHeldElsewhere(sessionId: string): boolean {
	const prefix = sessionId + ':'
	for (const key of remoteRuns.keys()) {
		if (key.startsWith(prefix)) return true
	}
	return false
}

let remoteTurnEnd: ((sessionId: string, chatId: string) => void | Promise<void>) | undefined

/** Registered by sessionRuntime, which already imports this module — a
 *  callback rather than an import keeps that edge one-way. The returned
 *  promise is when the catch-up has been applied; the composer stays locked
 *  until it settles. */
export function onRemoteTurnEnd(
	fn: (sessionId: string, chatId: string) => void | Promise<void>
): void {
	remoteTurnEnd = fn
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
	// A user switch rescopes the channel name, so the previous identity's
	// channel is closed before the next one opens.
	onUserChange(() => openChannel())
}

function receive(msg: SyncMsg): void {
	switch (msg.kind) {
		case 'run-heartbeat':
			remoteRuns.set(runKey(msg.sessionId, msg.from), { at: Date.now() })
			ensurePruner()
			break
		case 'turn-end': {
			// Unlocking on receipt would let a send here start from history missing
			// the turn that just ended, so the slot holds until the catch-up
			// settles — unless the driver's next turn replaced it meanwhile (object
			// identity, see remoteRuns). The pruner caps a wedged reload at STALE_MS.
			const key = runKey(msg.sessionId, msg.from)
			const hold = { at: Date.now() }
			remoteRuns.set(key, hold)
			ensurePruner()
			Promise.resolve()
				.then(() => remoteTurnEnd?.(msg.sessionId, msg.chatId))
				.catch((e) => console.error('sessionSync: turn-end handler failed', e))
				.finally(() => {
					if (remoteRuns.get(key) === hold) remoteRuns.delete(key)
				})
			break
		}
	}
}

function post(msg: SyncMsg): void {
	if (!channel) return
	try {
		channel.postMessage(msg)
	} catch (e) {
		// A failed post must never take the turn down with it.
		console.error('sessionSync: could not post message', e)
	}
}

let pruneTimer: ReturnType<typeof setInterval> | undefined

function ensurePruner(): void {
	if (pruneTimer) return
	pruneTimer = setInterval(() => {
		const cutoff = Date.now() - STALE_MS
		for (const [id, entry] of remoteRuns) {
			if (entry.at < cutoff) remoteRuns.delete(id)
		}
		if (remoteRuns.size === 0) {
			clearInterval(pruneTimer)
			pruneTimer = undefined
		}
	}, PRUNE_MS)
}

// ---------------------------------------------------------------------------
// Driving side
// ---------------------------------------------------------------------------

// The chat id rides along for the pagehide farewell below, which cannot ask
// the manager for it. Taken at run start; only a mid-turn rotation could make
// it stale, and a farewell pointing at the pre-rotation record still converges
// (the re-read is idempotent and the next turn-end names the right one).
const heartbeats = new Map<string, { timer: ReturnType<typeof setInterval>; chatId: string }>()

/** Posted when the run's loading bracket opens — after the send's attachment
 *  upkeep awaits, so a competing send can start during them; the sender's own
 *  post-preflight re-check is what refuses one that did. */
export function localRunStarted(sessionId: string, chatId: string): void {
	if (heartbeats.has(sessionId)) return
	post({ kind: 'run-heartbeat', sessionId, from: TAB_ID })
	heartbeats.set(sessionId, {
		timer: setInterval(
			() => post({ kind: 'run-heartbeat', sessionId, from: TAB_ID }),
			HEARTBEAT_MS
		),
		chatId
	})
}

/** `chatId` is read at turn end, not reused from the start: a rotation
 *  mid-turn means the transcript now lives under a different record, and the
 *  watchers' re-read must follow it there. */
export function localRunEnded(sessionId: string, chatId: string): void {
	const entry = heartbeats.get(sessionId)
	if (entry !== undefined) {
		clearInterval(entry.timer)
		heartbeats.delete(sessionId)
	}
	post({ kind: 'turn-end', sessionId, chatId, from: TAB_ID })
}

if (BROWSER) {
	// The run dies with the page: a turn-end farewell (which also has watchers
	// re-read the last checkpoint) beats making them wait out STALE_MS. Not on
	// a bfcache freeze (persisted) — that turn resumes with the page, and
	// nothing would re-arm a farewelled heartbeat.
	window.addEventListener('pagehide', (ev) => {
		if (ev.persisted) return
		for (const [sessionId, entry] of [...heartbeats]) {
			localRunEnded(sessionId, entry.chatId)
		}
	})
}

/** Test seam: deliver a message as if it arrived on the channel. */
export function __receiveForTest(msg: SyncMsg): void {
	receive(msg)
}

/** Test seam: clear the module's state between tests. */
export function __resetForTest(): void {
	remoteRuns.clear()
	if (pruneTimer) {
		clearInterval(pruneTimer)
		pruneTimer = undefined
	}
	for (const entry of heartbeats.values()) clearInterval(entry.timer)
	heartbeats.clear()
	remoteTurnEnd = undefined
}
