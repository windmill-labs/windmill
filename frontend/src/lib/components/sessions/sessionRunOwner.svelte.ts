import { BROWSER } from 'esm-env'
import { SvelteMap } from 'svelte/reactivity'
import { randomUUID } from '$lib/utils/uuid'

// Who holds a session's run, for every tab open on it.
//
// Two tabs sending on one session append to the same chat id, and whichever
// saveChat lands last silently drops the other's turn — after its tool calls
// have already run against the workspace. So one tab drives and the rest watch.
//
// Every gate in the cross-tab paths asks a variant of one question: may I send,
// may I save, is the run on screen still someone else's. This module answers all
// of them from a single position per session, so they cannot disagree with each
// other.

/** Whether this context can hold a real lock on a run. Web Locks is
 *  secure-context only, so a self-hosted instance served over plain HTTP has
 *  none — watching still works there, on the weaker footing described at
 *  {@link withSessionRunLock} and {@link runLockHeld}. */
const EXCLUSIVE_OWNERSHIP = BROWSER && !!globalThis.navigator?.locks?.query

/** A driver silent for this long is presumed gone, pending the lock query that
 *  confirms it. Deliberately many times the status cadence: a busy tab can be
 *  starved of ticks for a while without having gone anywhere. */
const DRIVER_SILENCE_MS = 10_000

/** Where this tab stands in one session's run.
 *
 *  `catchingUp` is its own position rather than a corner of `watching`: the
 *  driver's turn is over, but this tab still holds the conversation from before
 *  it, and sending from there would put one the driver has already moved past to
 *  the model. */
export type RunPosition =
	| { state: 'idle' }
	| { state: 'driving'; runId: string }
	| {
			state: 'watching'
			lastHeardAt: number
			/** Which of the driver's turns this tab is watching. A control message
			 *  names the run it was pressed for, so one delayed across a turn
			 *  boundary is dropped instead of landing on the turn that followed. */
			runId: string
			/** The driver's plan-mode posture, carried so a watching tab can show the
			 *  mode the run is actually under. It lives here rather than on the
			 *  manager so it cannot outlive the run it describes: the position moves
			 *  on at turn end and the posture goes with it. */
			planMode: boolean
	  }
	| { state: 'catchingUp' }

const IDLE: RunPosition = { state: 'idle' }

/** Reactive because the composer's locked state is derived from it. */
const positions = new SvelteMap<string, RunPosition>()

/** Sessions with no entry are idle, which is also the answer for a chat that
 *  has no session at all (the docked side-panel copilot). */
export function runPosition(sessionId: string | undefined): RunPosition {
	if (!sessionId) return IDLE
	return positions.get(sessionId) ?? IDLE
}

/** This tab is running the turn. */
export function isDriving(sessionId: string): boolean {
	return runPosition(sessionId).state === 'driving'
}

/** Another tab is running the turn and this one is showing that it is. */
export function isWatching(sessionId: string): boolean {
	return runPosition(sessionId).state === 'watching'
}

/** True from the driver's first status message until the re-read that follows
 *  its turn completes. The save and send paths gate on it: until it clears, what
 *  this tab holds is the conversation from before the driver's turn. */
export function runHeldElsewhere(sessionId: string | undefined): boolean {
	const state = runPosition(sessionId).state
	return state === 'watching' || state === 'catchingUp'
}

/** The driver said how its run is going, which is also its sign of life. */
export function noteDriverAlive(sessionId: string, planMode: boolean, runId: string): void {
	// A tab mid-turn is the authority on its own session; a status message
	// reaching it can only be an echo of the run it is itself driving.
	if (isDriving(sessionId)) return
	positions.set(sessionId, { state: 'watching', lastHeardAt: Date.now(), planMode, runId })
	ensureReaper()
}

/** The run this tab is in, either as its driver or as a watcher. Control
 *  messages carry it so they can only act on the turn they were meant for. */
export function currentRunId(sessionId: string): string | undefined {
	const p = runPosition(sessionId)
	return p.state === 'driving' || p.state === 'watching' ? p.runId : undefined
}

/** The driver says its turn is over. The re-read that follows is what frees this
 *  tab, so the position moves to `catchingUp` — but only where a runtime exists
 *  to perform it. The channel runs this in every tab that loads it, and one with
 *  no runtime would sit in a state nothing can leave. */
export function noteRemoteTurnEnded(sessionId: string): void {
	const state = runPosition(sessionId).state
	// `idle` counts. A tab that heard the turn end without having heard it start
	// — it opened between the driver's last status message and its turn-end, or
	// its handler registered in that window — holds a transcript from before the
	// turn, and idle is precisely the position that permits driving on it.
	if (state === 'driving' || state === 'catchingUp') return
	if (!canCompleteCatchUp()) {
		positions.delete(sessionId)
		return
	}
	positions.set(sessionId, { state: 'catchingUp' })
}

/** The re-read finished: this tab's transcript and history are one conversation
 *  again, and it may drive the next turn. */
export function noteCaughtUp(sessionId: string): void {
	if (runPosition(sessionId).state !== 'catchingUp') return
	positions.delete(sessionId)
}

/** Drop everything held for a session whose runtime is going away, so a torn
 *  down tab can't leave a position behind for a session nothing is watching. */
export function clearRunPosition(sessionId: string): void {
	positions.delete(sessionId)
}

/** True from the driver's turn-end until this tab has re-read what it left
 *  behind. The catch-up retry reads it to tell "still owed a read" from "a new
 *  turn started" and from "this session is gone". */
export function isCatchingUp(sessionId: string): boolean {
	return runPosition(sessionId).state === 'catchingUp'
}

function lockName(sessionId: string): string {
	return `wm-session-run:${sessionId}`
}

/** Run `body` as the session's sole driver, or return 'busy' without running it
 *  when another tab already holds the run. `body` runs at most once either way.
 *
 *  Web Locks is what makes this safe across a crash: the lock is held by the tab,
 *  not by a record someone has to clean up, so a driver that dies mid-turn
 *  releases it and the next send succeeds. Where it is missing or unusable,
 *  {@link bestEffort} takes over. */
export async function withSessionRunLock<T>(
	sessionId: string,
	body: () => Promise<T>
): Promise<T | 'busy'> {
	// Ahead of the lock, because holding the lock is not the same as being
	// entitled to the conversation: the driver releases it at turn-end while this
	// tab still owes itself the re-read, and a turn driven in that window sends
	// the pre-run history the re-read exists to replace. `runHeldElsewhere` and not
	// `isWatching` for exactly that reason.
	if (runHeldElsewhere(sessionId)) return 'busy'
	if (!EXCLUSIVE_OWNERSHIP) return await bestEffort(sessionId, body)
	// Set before the body runs, so a turn that throws is told apart from a lock
	// that could not be taken. Without it a failing turn would look like a failed
	// arbitration and be run a second time by the fallback below.
	let entered = false
	try {
		return (await navigator.locks.request(
			lockName(sessionId),
			{ mode: 'exclusive', ifAvailable: true },
			async (lock) => {
				// `ifAvailable` hands back a null lock instead of queueing when another
				// tab holds it, which is exactly the "refuse, don't stack up turns"
				// behavior we want.
				if (!lock) return 'busy' as const
				// Re-checked here because the grant is a turn of the event loop away
				// from the check above, and the driver's turn-end is both what frees
				// the lock and what lands the status making this tab a watcher — so
				// the one grant we must refuse is the one that arrives this way.
				if (runHeldElsewhere(sessionId)) return 'busy' as const
				entered = true
				return await drive(sessionId, body)
			}
		)) as T | 'busy'
	} catch (e) {
		if (entered) throw e
		// The lock API is present but refused to arbitrate. Degrade to the footing
		// a context without it already runs on rather than failing the turn: a
		// session the user cannot send in is a worse outcome than one whose
		// exclusion is best-effort for this send.
		console.error('sessionRunOwner: run lock unavailable, excluding best-effort instead', e)
		return await bestEffort(sessionId, body)
	}
}

/** What exclusion amounts to with no lock to take: refuse while another tab's
 *  run is visibly on screen, and otherwise go.
 *
 *  This is deliberately not mutual exclusion, and cannot be made into it. A
 *  driver whose timers have been throttled in a hidden tab goes silent long
 *  before its turn ends, so it is reaped as dead and a send from here can
 *  start a second turn against the same chat id. Nothing over the channel fixes
 *  that: a probe distinguishes a throttled tab from a closed one, but not a
 *  frozen tab from a closed one, and the browser freezes hidden tabs on much the
 *  same schedule as it throttles them.
 *
 *  Accepted rather than solved, because the only origins that land here are
 *  served over plain HTTP and are not localhost — a shape used for local testing
 *  rather than for running Windmill. Every HTTPS deployment, and localhost, is a
 *  secure context and takes the real lock above. */
async function bestEffort<T>(sessionId: string, body: () => Promise<T>): Promise<T | 'busy'> {
	return await drive(sessionId, body)
}

async function drive<T>(sessionId: string, body: () => Promise<T>): Promise<T> {
	positions.set(sessionId, { state: 'driving', runId: randomUUID() })
	try {
		return await body()
	} finally {
		// Straight to idle: this tab wrote the turn it just ran, so there is
		// nothing of anyone else's to catch up on.
		positions.delete(sessionId)
	}
}

/** Whether any tab currently holds the run lock for this session. Used to
 *  settle a driver that went silent: a released lock proves the tab is gone,
 *  where silence alone only suggests it. */
async function runLockHeld(sessionId: string): Promise<boolean> {
	// Nothing to consult without the lock API, so a silent driver is reaped on
	// silence alone — and on that path reaching `idle` does entitle this tab to
	// drive, with everything that implies when the driver was merely throttled.
	// See {@link bestEffort} for why that is accepted rather than defended.
	if (!EXCLUSIVE_OWNERSHIP) return false
	try {
		const state = await navigator.locks.query()
		const name = lockName(sessionId)
		return !!state.held?.some((l) => l.name === name)
	} catch {
		return true
	}
}

let driverLost: ((sessionId: string) => void) | undefined

/** Registered by sessionRuntime at module load, so this module stays free of
 *  its imports — the two would otherwise sit in a cycle. */
export function onDriverLost(fn: (sessionId: string) => void): void {
	driverLost = fn
}

/** sessionRuntime owns both the re-read and this handler, so registering one
 *  means having the other. */
function canCompleteCatchUp(): boolean {
	return driverLost !== undefined
}

// Runs only while some session is being driven elsewhere, and stops itself once
// none is — a browser with a single tab open never arms it at all.
let reaperTimer: ReturnType<typeof setInterval> | undefined

function ensureReaper(): void {
	if (reaperTimer) return
	reaperTimer = setInterval(() => {
		if (![...positions.values()].some((p) => p.state === 'watching')) {
			clearInterval(reaperTimer)
			reaperTimer = undefined
			return
		}
		void reapDeadDrivers()
	}, DRIVER_SILENCE_MS)
}

/** Release watchers whose driver went silent and no longer holds the lock, so a
 *  closed tab can't leave a session showing "generating" forever. */
async function reapDeadDrivers(): Promise<void> {
	const now = Date.now()
	const stale = [...positions.entries()]
		.filter(([, p]) => p.state === 'watching' && now - p.lastHeardAt > DRIVER_SILENCE_MS)
		.map(([id]) => id)
	for (const id of stale) {
		// Re-checked after the await: a status message may have landed while the
		// query was in flight, and reaping then would tear down a live run.
		if (await runLockHeld(id)) continue
		if (!isWatching(id)) continue
		noteRemoteTurnEnded(id)
		driverLost?.(id)
	}
}
