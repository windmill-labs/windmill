import { JobService } from '$lib/gen'

export type RunStatus = 'running' | 'success' | 'failure'
/** Per-runnable badge state: latest run status + runs observed this session. */
export type RunnableRunState = { status: RunStatus; runs: number }

/**
 * Tracks which pipeline runnables currently have an in-flight (queued/running)
 * job — plus any that started AND finished since the last poll, so a fast
 * cascade hop that completes between two ticks still pulses once. Returns a
 * reactive `Set<string>` of `${kind}:${path}` ids (matching the graph's
 * runnable node ids) for the canvas to animate.
 *
 * Cost model — deliberately cheap:
 *  - ZERO requests while idle. Polling only runs after `arm()` (a user-
 *    initiated run) and self-sustains only while jobs are in flight.
 *  - One folder-scoped, `perPage`-capped `listExtendedJobs` request per tick.
 *  - Auto-disarms after `MAX_IDLE_TICKS` consecutive empty ticks (rides the
 *    brief gaps between cascade hops), with a hard total-duration cap.
 *
 * Read-only (`listExtendedJobs` is a read endpoint, no editor dependency), so
 * a future read-only / operator pipeline view can reuse this as-is.
 */
export function useActiveRunnableIds(
	getWorkspace: () => string | undefined,
	getPathPrefix: () => string | undefined
) {
	const INTERVAL_MS = 3000
	const MAX_IDLE_TICKS = 3 // ~9s of no in-flight jobs before disarming
	const HARD_CAP_MS = 5 * 60_000 // safety: stop a runaway poll loop
	const PER_PAGE = 50 // folder-scoped + newest-first: plenty for a cascade
	// Small look-back so a job that finished right after the previous tick
	// fired (but before its response landed) isn't dropped from catch-up.
	const CATCHUP_OVERLAP_MS = 4000

	let ids = $state(new Set<string>())
	// Per-runnable last-run state + session run count, for the node badge.
	// Reassigned as a fresh Map each tick (reactive without SvelteMap).
	let states = $state(new Map<string, RunnableRunState>())
	let timer: ReturnType<typeof setTimeout> | undefined
	let running = false
	let idleTicks = 0
	let startedAtMs = 0
	let lastPollTs = new Date(0).toISOString()

	// Persistent (non-reactive) accumulators — survive `stop()` so the badge
	// keeps showing the last status while idle; only `dispose()` clears them.
	const completedHistory = new Map<
		string,
		{ runs: number; lastStatus: RunStatus; lastTs: string }
	>()
	const countedJobIds = new Set<string>()

	function idOf(jobKind: string | undefined, path: string | undefined): string | undefined {
		if (!path || !jobKind) return undefined
		const kind = jobKind.startsWith('flow') ? 'flow' : 'script'
		return `${kind}:${path}`
	}

	function stop() {
		if (timer !== undefined) clearTimeout(timer)
		timer = undefined
		running = false
		idleTicks = 0
		if (ids.size > 0) ids = new Set()
	}

	async function tick() {
		const ws = getWorkspace()
		const prefix = getPathPrefix()
		if (!ws || !prefix) {
			stop()
			return
		}
		const since = lastPollTs
		const pollStartedMs = Date.now()
		let next = new Set<string>()
		const inFlightThisTick = new Set<string>()
		let anyInFlight = false
		try {
			const res = await JobService.listExtendedJobs({
				workspace: ws,
				scriptPathStart: prefix,
				jobKinds: ['preview', 'script', 'flow', 'flowpreview', 'flowscript'].join(','),
				perPage: PER_PAGE,
				page: 1
			})
			for (const j of res.jobs ?? []) {
				const id = idOf((j as any).job_kind, (j as any).script_path)
				if (!id) continue
				if ((j as any).type === 'QueuedJob') {
					// queued or running — currently active
					next.add(id)
					inFlightThisTick.add(id)
					anyInFlight = true
				} else {
					// completed: catch-up — a hop that started after the last
					// poll (and already finished) still gets one pulse.
					const started: string | undefined = (j as any).started_at ?? (j as any).created_at
					if (started && started >= since) next.add(id)
					// Tally distinct completed jobs for the node badge. Jobs
					// come newest-first; only let a strictly newer completion
					// move the displayed status.
					const jobId: string | undefined = (j as any).id
					if (jobId && !countedJobIds.has(jobId)) {
						countedJobIds.add(jobId)
						const prev = completedHistory.get(id)
						const status: RunStatus = (j as any).success === true ? 'success' : 'failure'
						const ts = started ?? new Date(pollStartedMs).toISOString()
						completedHistory.set(id, {
							runs: (prev?.runs ?? 0) + 1,
							lastStatus: !prev || ts >= prev.lastTs ? status : prev.lastStatus,
							lastTs: !prev || ts >= prev.lastTs ? ts : prev.lastTs
						})
					}
				}
			}
		} catch {
			// Transient failure: keep the previous set, retry next tick.
			next = ids
			anyInFlight = ids.size > 0
		}
		ids = next
		// Rebuild the badge snapshot: in-flight wins (spinner), otherwise the
		// last completed status. Fresh Map each tick → reactive without
		// SvelteMap. completedHistory persists across `stop()`.
		const snap = new Map<string, RunnableRunState>()
		for (const [id, h] of completedHistory) {
			snap.set(id, { status: inFlightThisTick.has(id) ? 'running' : h.lastStatus, runs: h.runs })
		}
		for (const id of inFlightThisTick) {
			if (!snap.has(id)) snap.set(id, { status: 'running', runs: 0 })
		}
		states = snap
		// Look back slightly so a job finishing in the request window isn't
		// missed by the next tick's catch-up comparison.
		lastPollTs = new Date(pollStartedMs - CATCHUP_OVERLAP_MS).toISOString()

		if (anyInFlight) idleTicks = 0
		else idleTicks++

		const expired = Date.now() - startedAtMs > HARD_CAP_MS
		if (idleTicks > MAX_IDLE_TICKS || expired) {
			stop()
			return
		}
		timer = setTimeout(() => void tick(), INTERVAL_MS)
	}

	return {
		get ids() {
			return ids
		},
		/** Per-runnable last-run status + session run count, for node badges. */
		get states() {
			return states
		},
		/**
		 * Kick the poll loop (call when a run is launched from this view).
		 * Idempotent: if already polling, just resets the idle counter so the
		 * loop keeps going to cover the freshly-launched run + its cascade.
		 */
		arm() {
			idleTicks = 0
			if (running) return
			running = true
			startedAtMs = Date.now()
			// Open the catch-up window from now so we don't replay old history.
			lastPollTs = new Date(Date.now() - CATCHUP_OVERLAP_MS).toISOString()
			void tick()
		},
		/**
		 * Stop polling and fully reset (call on destroy / folder change).
		 * Unlike `stop()` this also wipes the persisted badge history so a
		 * different folder doesn't inherit the previous one's run state.
		 */
		dispose() {
			stop()
			completedHistory.clear()
			countedJobIds.clear()
			if (states.size > 0) states = new Map()
		}
	}
}
