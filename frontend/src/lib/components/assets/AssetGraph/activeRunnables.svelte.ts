import { JobService } from '$lib/gen'

export type RunStatus = 'running' | 'success' | 'failure'
/**
 * Per-runnable badge state: latest run status + runs observed this session.
 * `lastSuccessAt` is the start time of the newest successful run seen by the
 * poll — lets the freshness chip go green right after an in-session run,
 * ahead of the next graph refetch (whose `last_success_at` would carry it).
 */
export type RunnableRunState = { status: RunStatus; runs: number; lastSuccessAt?: string }

export type EventStatus = 'queued' | 'running' | 'success' | 'failure'
/** One folder activity-log row (a job observed by the poll). */
export type PipelineEvent = {
	id: string
	path: string
	kind: 'script' | 'flow'
	status: EventStatus
	/** What started it, as far as the job listing reveals. */
	source: 'schedule' | 'run'
	at: string
	/**
	 * Queued jobs: when the job is due to start. A future value means a
	 * scheduled run waiting for its cron tick, not pipeline activity.
	 */
	scheduledFor?: string
}

/**
 * Whether an event counts toward the "N running" badges: running, or queued
 * and already due. A queued job whose `scheduled_for` is in the future is the
 * schedule's next planned run — nothing is executing.
 */
export function isActiveEvent(e: PipelineEvent, now = Date.now()): boolean {
	if (e.status === 'running') return true
	if (e.status !== 'queued') return false
	return !e.scheduledFor || new Date(e.scheduledFor).getTime() <= now
}
const MAX_EVENTS = 50

// Equality guards so an unchanged poll tick doesn't reassign the reactive
// state — a no-op `states`/`ids` reassignment would otherwise re-derive the
// whole canvas (full d3-dag Sugiyama layout + edge re-route) every 3–6s.
function setEq(a: Set<string>, b: Set<string>): boolean {
	if (a.size !== b.size) return false
	for (const v of a) if (!b.has(v)) return false
	return true
}
function statesEq(a: Map<string, RunnableRunState>, b: Map<string, RunnableRunState>): boolean {
	if (a.size !== b.size) return false
	for (const [k, v] of a) {
		const w = b.get(k)
		if (!w || w.status !== v.status || w.runs !== v.runs || w.lastSuccessAt !== v.lastSuccessAt)
			return false
	}
	return true
}
function eventsEq(a: PipelineEvent[], b: PipelineEvent[]): boolean {
	if (a.length !== b.length) return false
	for (let i = 0; i < a.length; i++) {
		if (a[i].id !== b[i].id || a[i].status !== b[i].status) return false
	}
	return true
}

/**
 * Polls folder jobs and exposes, as reactive `${kind}:${path}`-keyed state:
 * the in-flight set (`ids`, incl. a one-tick pulse for hops that start and
 * finish between ticks), per-runnable badge state (`states`), and a capped
 * activity log (`events`).
 *
 * Polling runs after `arm()` (a launched run, fast cadence) or while
 * `setObserving(true)` (slow cadence — the page keeps this on the whole time
 * the graph is open so node badges stay live). The armed window self-disarms
 * when idle; the observe window stops on `setObserving(false)` / `dispose()`.
 * Read-only (`listExtendedJobs`), so a future operator view can reuse it.
 */
export function useActiveRunnableIds(
	getWorkspace: () => string | undefined,
	getPathPrefix: () => string | undefined
) {
	const INTERVAL_MS = 3000
	// Slower cadence when only the activity log is open (passive watching,
	// nothing launched from here) — keeps the idle observe cost low.
	const OBSERVE_INTERVAL_MS = 6000
	const MAX_IDLE_TICKS = 3 // ~9s of no in-flight jobs before disarming
	const HARD_CAP_MS = 5 * 60_000 // safety: stop a runaway poll loop
	const PER_PAGE = 50 // folder-scoped + newest-first: plenty for a cascade
	// Small look-back so a job that finished right after the previous tick
	// fired (but before its response landed) isn't dropped from catch-up.
	const CATCHUP_OVERLAP_MS = 4000
	// Baseline for "this session": runs before the graph was opened are
	// pre-existing history, not something the user did here — exclude them
	// from the node run-count and the activity log so both read as "since
	// you opened this graph".
	const openedAtIso = new Date().toISOString()

	let ids = $state(new Set<string>())
	// Per-runnable last-run state + session run count, for the node badge.
	// Reassigned as a fresh Map each tick (reactive without SvelteMap).
	let states = $state(new Map<string, RunnableRunState>())
	// Newest-first capped activity log; reassigned each tick (reactive).
	let events = $state<PipelineEvent[]>([])
	let timer: ReturnType<typeof setTimeout> | undefined
	let running = false
	// `arm()` opens a self-terminating fast-poll window; `observing` keeps a
	// slower poll alive while the activity-log panel is open.
	let observing = false
	let armedActive = false
	let idleTicks = 0
	let startedAtMs = 0
	let lastPollTs = new Date(0).toISOString()
	// Single-flight guard. `arm()` calls `tick()` directly; without this, an
	// arm landing while a tick's request is still outstanding starts a SECOND
	// concurrent fetch — and both then schedule their own setTimeout chain
	// (the leaked chain's timer is no longer in `timer`, so stop()/arm()
	// can't clear it). Every launch (each cascade hop arms once) added a
	// chain, compounding into many concurrent identical list_jobs calls.
	// With the guard, at most one request is ever in flight; an arm that
	// lands mid-flight just queues one immediate re-poll.
	let inFlight = false
	let rerunAsap = false
	// Bumped by stop(): a tick whose fetch resolves after its loop was
	// stopped (folder change / dispose) must not write stale results into
	// the new scope or schedule itself back to life.
	let pollGen = 0

	function schedule(delayMs: number) {
		if (timer !== undefined) clearTimeout(timer)
		timer = setTimeout(() => void tick(), delayMs)
	}

	// Persistent (non-reactive) accumulators — survive `stop()` so the badge
	// keeps showing the last status while idle; only `dispose()` clears them.
	const completedHistory = new Map<
		string,
		{ runs: number; lastStatus: RunStatus; lastTs: string; lastSuccessTs?: string }
	>()
	const countedJobIds = new Set<string>()
	// Job ids we've observed in-flight at least once. The catch-up pulse is
	// only for jobs whose whole start→finish fell between two polls (never
	// seen running); re-pulsing a job we already animated while it ran just
	// keeps its edges lit ~one extra poll interval after it finished.
	const seenInFlightJobIds = new Set<string>()
	// Runnable ids launched from this view (via `arm(id)`). Their run is
	// already animated zero-latency by the page's `activeRunnable` hint, so
	// the catch-up pulse must not re-flash them after completion even if the
	// poll never sampled their in-flight window. Cascade hops have different
	// ids and still pulse. Cleared when the poll fully stops.
	const launchedIds = new Set<string>()
	// jobId → latest known state of that job, for the activity log. Survives
	// `stop()` (log keeps history while idle); cleared by `dispose()`.
	const eventsById = new Map<string, PipelineEvent>()

	function idOf(jobKind: string | undefined, path: string | undefined): string | undefined {
		if (!path || !jobKind) return undefined
		const kind = jobKind.startsWith('flow') ? 'flow' : 'script'
		return `${kind}:${path}`
	}

	function stop() {
		pollGen++
		rerunAsap = false
		if (timer !== undefined) clearTimeout(timer)
		timer = undefined
		running = false
		armedActive = false
		idleTicks = 0
		launchedIds.clear()
		if (ids.size > 0) ids = new Set()
	}

	async function tick() {
		if (inFlight) {
			// A request is already outstanding — don't stack another; the
			// running tick re-polls immediately when it lands.
			rerunAsap = true
			return
		}
		const ws = getWorkspace()
		const prefix = getPathPrefix()
		if (!ws || !prefix) {
			stop()
			return
		}
		inFlight = true
		const gen = pollGen
		const since = lastPollTs
		const pollStartedMs = Date.now()
		let next = new Set<string>()
		const runningThisTick = new Set<string>()
		let anyInFlight = false
		try {
			const res = await JobService.listExtendedJobs({
				workspace: ws,
				scriptPathStart: prefix,
				jobKinds: ['preview', 'script', 'flow', 'flowpreview', 'flowscript'].join(','),
				perPage: PER_PAGE,
				page: 1
			})
			// Stopped (folder change / dispose) while the request was in the
			// air — drop the stale response and let the loop die here.
			if (gen !== pollGen) return
			for (const j of res.jobs ?? []) {
				const id = idOf((j as any).job_kind, (j as any).script_path)
				if (!id) continue
				const jobId: string | undefined = (j as any).id
				const startedTs: string | undefined = (j as any).started_at ?? (j as any).created_at
				// `type === 'QueuedJob'` covers both queued-and-waiting *and*
				// actually-executing jobs. Only the latter (`running === true`)
				// should drive the "running" badge / edge animation — queued
				// jobs haven't started yet, so the node should keep its prior
				// status until a worker picks it up. We still keep the queued
				// row alive for the activity log (built further down) and treat
				// it as "in flight" for poll-cadence purposes so the loop
				// doesn't disarm while runs are pending.
				const isQueued = (j as any).type === 'QueuedJob'
				const isRunning = isQueued && (j as any).running === true
				if (isRunning) {
					next.add(id)
					runningThisTick.add(id)
					anyInFlight = true
					if (jobId) seenInFlightJobIds.add(jobId)
				} else if (isQueued) {
					// Queued but not yet running — keep the poll alive but
					// don't surface this as an executing runnable.
					anyInFlight = true
				} else {
					// completed: catch-up — a hop whose whole lifetime fell
					// between two polls (never observed in-flight) still gets
					// one pulse. Jobs we already animated while running are
					// NOT re-pulsed, else their edges linger ~a poll interval
					// past completion.
					if (
						startedTs &&
						startedTs >= since &&
						(!jobId || !seenInFlightJobIds.has(jobId)) &&
						!launchedIds.has(id)
					)
						next.add(id)
					// Tally distinct completed jobs for the node badge — only
					// those since the graph was opened (older = pre-existing
					// history). Jobs come newest-first; only a strictly newer
					// completion moves the displayed status.
					if (jobId && !countedJobIds.has(jobId) && (startedTs ?? '') >= openedAtIso) {
						countedJobIds.add(jobId)
						const prev = completedHistory.get(id)
						const status: RunStatus = (j as any).success === true ? 'success' : 'failure'
						const ts = startedTs ?? new Date(pollStartedMs).toISOString()
						completedHistory.set(id, {
							runs: (prev?.runs ?? 0) + 1,
							lastStatus: !prev || ts >= prev.lastTs ? status : prev.lastStatus,
							lastTs: !prev || ts >= prev.lastTs ? ts : prev.lastTs,
							lastSuccessTs:
								status === 'success' && (!prev?.lastSuccessTs || ts >= prev.lastSuccessTs)
									? ts
									: prev?.lastSuccessTs
						})
					}
				}
				// Activity-log row: upsert latest known state (a job seen
				// queued then completed on a later tick updates in place).
				// In-flight jobs always show (relevant now even if they
				// started just before open); completed ones only if they ran
				// since the graph was opened.
				if (jobId && (isQueued || (startedTs ?? '') >= openedAtIso)) {
					eventsById.set(jobId, {
						id: jobId,
						path: (j as any).script_path ?? '',
						kind: String((j as any).job_kind ?? '').startsWith('flow') ? 'flow' : 'script',
						status: isQueued
							? (j as any).running === true
								? 'running'
								: 'queued'
							: (j as any).success === true
								? 'success'
								: 'failure',
						source: (j as any).schedule_path ? 'schedule' : 'run',
						at: startedTs ?? new Date(pollStartedMs).toISOString(),
						scheduledFor: isQueued ? ((j as any).scheduled_for as string | undefined) : undefined
					})
				}
			}
		} catch {
			// Transient failure: keep the previous set, retry next tick.
			next = ids
			anyInFlight = ids.size > 0
		} finally {
			inFlight = false
		}
		// Same stale-loop check for the catch path — a failed fetch must not
		// resurrect a stopped loop via the scheduling below.
		if (gen !== pollGen) return
		if (!setEq(ids, next)) ids = next
		// Rebuild the badge snapshot: actually-running wins (spinner),
		// otherwise the last completed status sticks. completedHistory
		// persists across `stop()`. Queued-but-not-started jobs are
		// intentionally NOT surfaced as 'running' — the node keeps its
		// previous badge state until a worker picks the job up.
		const snap = new Map<string, RunnableRunState>()
		for (const [id, h] of completedHistory) {
			snap.set(id, {
				status: runningThisTick.has(id) ? 'running' : h.lastStatus,
				runs: h.runs,
				lastSuccessAt: h.lastSuccessTs
			})
		}
		for (const id of runningThisTick) {
			if (!snap.has(id)) snap.set(id, { status: 'running', runs: 0 })
		}
		if (!statesEq(states, snap)) states = snap
		// Activity-log snapshot: newest-first, capped. Prune the backing map
		// (and the dedup set in lockstep) so a long session stays bounded.
		const sorted = Array.from(eventsById.values()).sort((a, b) => b.at.localeCompare(a.at))
		if (sorted.length > MAX_EVENTS * 4) {
			for (const e of sorted.slice(MAX_EVENTS * 4)) eventsById.delete(e.id)
			countedJobIds.clear()
			seenInFlightJobIds.clear()
			for (const id of eventsById.keys()) {
				countedJobIds.add(id)
				seenInFlightJobIds.add(id)
			}
		}
		const nextEvents = sorted.slice(0, MAX_EVENTS)
		if (!eventsEq(events, nextEvents)) events = nextEvents
		// Look back slightly so a job finishing in the request window isn't
		// missed by the next tick's catch-up comparison.
		lastPollTs = new Date(pollStartedMs - CATCHUP_OVERLAP_MS).toISOString()

		if (anyInFlight) idleTicks = 0
		else idleTicks++

		const expired = Date.now() - startedAtMs > HARD_CAP_MS
		if (idleTicks > MAX_IDLE_TICKS || expired) armedActive = false
		// Keep polling while a fast armed window is open OR the activity log
		// is being watched; otherwise wind down to zero requests.
		if (!armedActive && !observing) {
			stop()
			return
		}
		// An arm() that landed mid-flight queued an immediate re-poll;
		// otherwise fall back to the cadence. schedule() clears any pending
		// timer first, so there is always exactly one chain.
		const asap = rerunAsap
		rerunAsap = false
		schedule(asap ? 0 : armedActive ? INTERVAL_MS : OBSERVE_INTERVAL_MS)
	}

	return {
		get ids() {
			return ids
		},
		/** Per-runnable last-run status + session run count, for node badges. */
		get states() {
			return states
		},
		/** Newest-first capped activity log of folder jobs observed. */
		get events() {
			return events
		},
		/**
		 * Kick the fast poll window (call when a run is launched from this
		 * view). Reopens the self-terminating fast window and polls now;
		 * upgrades cadence if only the slow observe poll was running.
		 * `launchedId` (`${kind}:${path}`) marks the runnable the page
		 * animates zero-latency itself, so the catch-up pulse won't re-flash
		 * it after completion.
		 */
		arm(launchedId?: string) {
			if (launchedId) launchedIds.add(launchedId)
			armedActive = true
			idleTicks = 0
			startedAtMs = Date.now()
			// Open the catch-up window from now so we don't replay old history.
			lastPollTs = new Date(Date.now() - CATCHUP_OVERLAP_MS).toISOString()
			if (timer !== undefined) {
				clearTimeout(timer)
				timer = undefined
			}
			running = true
			void tick()
		},
		/**
		 * Toggle the activity-log observe poll. While on, the loop keeps
		 * polling at the slow cadence even with no run launched here, so the
		 * log updates live; while off (and no armed window) polling stops.
		 */
		setObserving(v: boolean) {
			observing = v
			if (v) {
				if (!running) {
					running = true
					lastPollTs = new Date(Date.now() - CATCHUP_OVERLAP_MS).toISOString()
					void tick()
				}
			} else if (!armedActive) {
				stop()
			}
		},
		/**
		 * Stop polling and fully reset (call on destroy / folder change).
		 * Unlike `stop()` this also wipes the persisted badge/log history so a
		 * different folder doesn't inherit the previous one's run state.
		 */
		dispose() {
			observing = false
			stop()
			completedHistory.clear()
			countedJobIds.clear()
			seenInFlightJobIds.clear()
			eventsById.clear()
			if (states.size > 0) states = new Map()
			if (events.length > 0) events = []
		}
	}
}
