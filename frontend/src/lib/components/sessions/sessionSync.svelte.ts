import { BROWSER } from 'esm-env'
import { SvelteMap } from 'svelte/reactivity'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'

// Cross-tab awareness for AI sessions, deliberately minimal. Two facts cross
// the channel: "a turn is running here" (a repeated heartbeat) and "the turn
// ended" (posted after its last IndexedDB write). Watching tabs lock the
// session's composer while the first holds, and re-read the shared chat record
// on the second — no transcript, run state, or session record ever rides a
// message, so tabs converge on what the store holds rather than on delivery
// order.
//
// The lock is advisory: nothing arbitrates two sends racing inside broadcast
// latency, and the loser's IndexedDB write stands until the next turn-end
// re-read — the same last-writer-wins two tabs had before any channel existed.
//
// The channel is per-user (same email scoping as the IndexedDB stores), so a
// browser shared by two accounts never crosses them.

const CHANNEL_BASE = 'windmill-sessions-sync'

/** Re-posted while the turn runs so silence identifies a driver that died
 *  without ending its turn: watchers unlock once nothing has arrived for
 *  STALE_MS. The window sits above the once-per-minute floor browsers throttle
 *  a hidden tab's timers to — and a hidden driver is the normal case, since
 *  watching from the other tab is what this module exists for. A cleanly
 *  closed driver unlocks fast via the pagehide farewell below; the full wait
 *  is only for a tab that died with no chance to say so. */
const HEARTBEAT_MS = 3_000
const STALE_MS = 90_000
const PRUNE_MS = 2_000

type SyncMsg =
	| { kind: 'run-heartbeat'; sessionId: string }
	| { kind: 'turn-end'; sessionId: string; chatId: string }

// Sessions running in OTHER tabs, by last heartbeat arrival. A tab never
// receives its own posts (BroadcastChannel does not echo to the poster), so
// presence alone means another tab; reactive readers (the composer lock)
// re-evaluate on every map change, the pruner's deletes included. The value
// is a fresh object per message on purpose: turn-end's deferred cleanup asks
// "is this entry still mine?" by identity, which a timestamp cannot answer —
// a follow-up heartbeat in the same millisecond would compare equal and be
// deleted, unlocking the composer under the driver's next turn.
const remoteRuns = new SvelteMap<string, { at: number }>()

export function runHeldElsewhere(sessionId: string): boolean {
	return remoteRuns.has(sessionId)
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
			remoteRuns.set(msg.sessionId, { at: Date.now() })
			ensurePruner()
			break
		case 'turn-end': {
			// The re-read is what makes unlocking safe: releasing on receipt would
			// open a gap where a send from this tab starts from history missing the
			// very turn that just ended. So the entry is refreshed for the reload's
			// duration and dropped only once the catch-up settles — unless a newer
			// message (the driver's next turn) has replaced the slot meanwhile,
			// detected by object identity (see remoteRuns). The pruner still caps
			// a wedged reload at STALE_MS.
			const hold = { at: Date.now() }
			remoteRuns.set(msg.sessionId, hold)
			ensurePruner()
			Promise.resolve()
				.then(() => remoteTurnEnd?.(msg.sessionId, msg.chatId))
				.catch((e) => console.error('sessionSync: turn-end handler failed', e))
				.finally(() => {
					if (remoteRuns.get(msg.sessionId) === hold) remoteRuns.delete(msg.sessionId)
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

/** Posted synchronously from the run's first state change, before its first
 *  await, so the window in which another tab can start a competing send is
 *  broadcast latency alone. */
export function localRunStarted(sessionId: string, chatId: string): void {
	if (heartbeats.has(sessionId)) return
	post({ kind: 'run-heartbeat', sessionId })
	heartbeats.set(sessionId, {
		timer: setInterval(() => post({ kind: 'run-heartbeat', sessionId }), HEARTBEAT_MS),
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
	post({ kind: 'turn-end', sessionId, chatId })
}

if (BROWSER) {
	// The run dies with the page, so watchers are told instead of being left to
	// the STALE_MS window. A turn-end (not a bespoke goodbye) also has them
	// re-read whatever the interrupted turn last checkpointed. Fires on bfcache
	// navigation too; a page restored mid-run re-arms on its next turn.
	window.addEventListener('pagehide', () => {
		for (const [sessionId, entry] of [...heartbeats]) {
			localRunEnded(sessionId, entry.chatId)
		}
	})
}
