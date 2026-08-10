/**
 * Synthesizes JobLoader replay streams from a run's completed jobs.
 *
 * A recording stores only what the run factually was: each completed job with
 * its timestamps, logs and result. At play time this module turns those into
 * the event streams a live run would have produced — logs distributed evenly
 * across each job's execution window, a flow's per-module statuses flipping at
 * the moments its sub-jobs started and finished — so the players and every
 * component under them (FlowStatusViewer, LogViewer, …) behave exactly as they
 * do while watching a real job.
 *
 * All `t` values are on the recording clock: zero = the root job's start.
 * Absolute timestamps inside the emitted jobs are rebased to "now" because
 * components like TimelineCompute measure elapsed time against `Date.now()`.
 *
 * Two hard rules, because recordings are caller-controlled (uploaded files,
 * `?src=` URLs on the public replay page) and this runs before any timer fires:
 *  - the input jobs are never mutated — every emitted job/status is a clone, so
 *    replaying twice from the same recording animates identically;
 *  - synthesis is budgeted across the whole replay (event count and cloned
 *    structure), so a file well inside the load-time render budgets still
 *    cannot make this loop materialize an unbounded amount of work.
 */
import type { FlowStatus, FlowStatusModule, Job } from '$lib/gen'
import type { ActiveReplayData, RecordedEvent, RecordedJob } from './types'

/** Target pacing of synthesized log chunks. */
const LOG_TICK_MS = 200
/** Cap on log events per job — each becomes a `setTimeout` scheduled up front. */
const MAX_LOG_TICKS = 300
/** Cap on flow-status snapshots per flow job, same reason. */
const MAX_STATUS_SNAPSHOTS = 500
/** Recorded timestamps are caller-controlled, so no event may schedule beyond
 * this horizon. */
const MAX_REPLAY_MS = 6 * 60 * 60 * 1000
/** Aggregate cap on synthesized events across every job of one replay — the
 * per-job caps alone would let a 2000-job recording materialize ~1.6M timer
 * events. Matches the total the v1 format enforced on stored events. */
const MAX_TOTAL_REPLAY_EVENTS = 20_000
/** Aggregate cap on the structure snapshots may clone: each flow-status
 * snapshot is a deep clone of the job's status, so a status near the per-value
 * render budget (~100k nodes) must get few snapshots, not 500. */
const MAX_TOTAL_SNAPSHOT_NODES = 500_000

/** Shared across every stream of one replay; see the caps above. */
type SynthesisBudget = { events: number; snapshotNodes: number }

function newBudget(): SynthesisBudget {
	return { events: MAX_TOTAL_REPLAY_EVENTS, snapshotNodes: MAX_TOTAL_SNAPSHOT_NODES }
}

const asArray = <T>(v: unknown): T[] => (Array.isArray(v) ? (v as T[]) : [])

function parseMs(d: unknown): number | undefined {
	if (typeof d !== 'string' && typeof d !== 'number') return undefined
	const t = new Date(d).getTime()
	return Number.isFinite(t) ? t : undefined
}

const clampT = (t: number) => Math.min(Math.max(Number.isFinite(t) ? t : 0, 0), MAX_REPLAY_MS)

function jobStartMs(job: Job | undefined): number | undefined {
	return parseMs(job?.started_at) ?? parseMs(job?.created_at)
}

function jobDurationMs(job: Job): number {
	const d = (job as any).duration_ms
	return typeof d === 'number' && Number.isFinite(d) && d > 0 ? Math.min(d, MAX_REPLAY_MS) : 0
}

function offsetDate(d: string | undefined, offset: number): string | undefined {
	if (!d) return d
	const t = new Date(d).getTime()
	return isNaN(t) ? d : new Date(t + offset).toISOString()
}

/** Mutates `fs` — only ever call on a clone this module created. */
function rebaseFlowStatus(fs: any, offset: number) {
	if (!fs) return
	const mods = [...asArray<any>(fs.modules), fs.failure_module, fs.preprocessor_module]
	for (const mod of mods) {
		const durations = mod?.flow_jobs_duration
		if (Array.isArray(durations?.started_at)) {
			durations.started_at = durations.started_at.map((d: string) => offsetDate(d, offset) ?? d)
		}
	}
}

/** Shift a job's absolute timestamps by `offset` ms so elapsed-time displays
 * anchored on `Date.now()` read correctly during replay. Mutates `job` — only
 * ever call on a clone this module created. */
function rebaseJob(job: any, offset: number) {
	for (const k of ['started_at', 'created_at', 'completed_at', 'scheduled_for']) {
		if (job[k]) job[k] = offsetDate(job[k], offset)
	}
	rebaseFlowStatus(job.flow_status, offset)
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v))

/** Rough structure size of a value, for the snapshot budget. Iterative so a
 * hostile depth can't blow the stack; bails once far past the budget. */
function roughNodeCount(v: unknown): number {
	let n = 0
	const stack: unknown[] = [v]
	while (stack.length > 0) {
		const x = stack.pop()
		if (Array.isArray(x)) {
			n += x.length
			for (const i of x) stack.push(i)
		} else if (x !== null && typeof x === 'object') {
			const keys = Object.keys(x)
			n += keys.length
			for (const k of keys) stack.push((x as any)[k])
		}
		if (n > 2 * MAX_TOTAL_SNAPSHOT_NODES) return n
	}
	return n
}

/** A module's execution window on the recording clock, from its recorded
 * per-iteration durations, its iteration jobs, or its single job. Undefined
 * when the recording carries no timing for it (the module then reveals its
 * final state only when its own job's — or the flow's — completion lands). */
function moduleSpan(
	mod: FlowStatusModule,
	jobsById: Record<string, Job>,
	anchorMs: number
): { start: number; end: number; iterStarts: number[] } | undefined {
	const starts: number[] = []
	const ends: number[] = []
	const fjd: any = mod.flow_jobs_duration
	const fjdStarts = asArray<unknown>(fjd?.started_at)
	if (fjdStarts.length > 0) {
		fjdStarts.forEach((s, i) => {
			const st = parseMs(s)
			if (st === undefined) return
			const d = Array.isArray(fjd?.duration_ms) ? fjd.duration_ms[i] : undefined
			starts.push(st - anchorMs)
			ends.push(st - anchorMs + (typeof d === 'number' && Number.isFinite(d) && d > 0 ? d : 0))
		})
	} else if (Array.isArray(mod.flow_jobs) && mod.flow_jobs.length > 0) {
		for (const jid of mod.flow_jobs) {
			const j = typeof jid === 'string' ? jobsById[jid] : undefined
			const st = jobStartMs(j)
			if (j === undefined || st === undefined) continue
			starts.push(st - anchorMs)
			ends.push(st - anchorMs + jobDurationMs(j))
		}
	} else if (typeof mod.job === 'string' && jobsById[mod.job]) {
		const j = jobsById[mod.job]
		const st = jobStartMs(j)
		if (st !== undefined) {
			starts.push(st - anchorMs)
			ends.push(st - anchorMs + jobDurationMs(j))
		}
	}
	if (starts.length === 0) return undefined
	const iterStarts = [...starts].sort((a, b) => a - b).map(clampT)
	return { start: clampT(Math.min(...starts)), end: clampT(Math.max(...ends)), iterStarts }
}

/** The module as it looked at time `T`: its final state once its window has
 * passed, a bare waiting marker before it, and in between an in-progress
 * variant with loop iterations trimmed to those already started. `orig` is
 * read-only timing input; the returned module is always freshly owned (built
 * from `cloned`, a clone of `orig`) so later rebasing can't touch the input. */
function moduleStateAt(
	T: number,
	orig: FlowStatusModule,
	cloned: FlowStatusModule | undefined,
	jobsById: Record<string, Job>,
	anchorMs: number
): FlowStatusModule {
	const span = moduleSpan(orig, jobsById, anchorMs)
	// No timing info (sub-job not recorded): keep the module hidden — its final
	// state arrives with the flow job's own completed event. Revealing it early
	// would also leak its `job` id, triggering sub-job discovery in the viewer.
	if (!span || T < span.start) {
		return { id: orig.id, type: 'WaitingForPriorSteps' }
	}
	const m: any = cloned ?? clone(orig)
	if (T >= span.end) return m
	const started = span.iterStarts.filter((s) => s <= T).length
	m.type = 'InProgress'
	if (Array.isArray(m.flow_jobs)) {
		m.flow_jobs = m.flow_jobs.slice(0, started)
		if (Array.isArray(m.flow_jobs_success)) {
			m.flow_jobs_success = m.flow_jobs_success.slice(0, started)
		}
		const fjd = m.flow_jobs_duration
		if (fjd) {
			if (Array.isArray(fjd.started_at)) fjd.started_at = fjd.started_at.slice(0, started)
			if (Array.isArray(fjd.duration_ms)) fjd.duration_ms = fjd.duration_ms.slice(0, started)
		}
		if (m.iterator !== null && typeof m.iterator === 'object') {
			m.iterator.index = Math.max(0, started - 1)
		}
	}
	return m
}

/** The whole flow status as it looked at time `T`, derived from the completed
 * status plus the sub-jobs' timings. Always a fresh deep structure. */
function flowStatusAt(
	T: number,
	final: FlowStatus,
	jobsById: Record<string, Job>,
	anchorMs: number
): FlowStatus {
	const fs: any = clone(final)
	const origModules = asArray<FlowStatusModule>(final.modules)
	const clonedModules = asArray<FlowStatusModule>(fs.modules)
	fs.modules = origModules.map((mod, i) =>
		moduleStateAt(T, mod, clonedModules[i], jobsById, anchorMs)
	)
	let step = 0
	for (const mod of origModules) {
		const span = moduleSpan(mod, jobsById, anchorMs)
		if (span && T >= span.end) step++
		else break
	}
	fs.step = step
	if (final.failure_module) {
		fs.failure_module = moduleStateAt(
			T,
			final.failure_module,
			fs.failure_module,
			jobsById,
			anchorMs
		)
	}
	if (final.preprocessor_module) {
		fs.preprocessor_module = moduleStateAt(
			T,
			final.preprocessor_module,
			fs.preprocessor_module,
			jobsById,
			anchorMs
		)
	}
	return fs
}

/** The times at which some module's shown state changes — each one gets a
 * flow-status snapshot event, thinned evenly to `max`. */
function statusBoundaries(
	final: FlowStatus,
	jobsById: Record<string, Job>,
	anchorMs: number,
	max: number
): number[] {
	if (max <= 0) return []
	const ts = new Set<number>()
	const mods = [
		...asArray<FlowStatusModule>(final.modules),
		final.failure_module,
		final.preprocessor_module
	]
	for (const mod of mods) {
		if (!mod) continue
		const span = moduleSpan(mod, jobsById, anchorMs)
		if (!span) continue
		ts.add(span.start)
		ts.add(span.end)
		for (const s of span.iterStarts) ts.add(s)
	}
	let sorted = [...ts].sort((a, b) => a - b)
	if (sorted.length > max) {
		const stride = sorted.length / max
		sorted = Array.from({ length: max }, (_, i) => sorted[Math.floor(i * stride)])
	}
	return sorted
}

/** Split `logs` into up to `maxTicks` chunks that concatenate back to the
 * original, cut on line boundaries where possible. */
function chunkLogs(logs: string, maxTicks: number): string[] {
	const lines = logs.match(/[^\n]*\n|[^\n]+$/g) ?? []
	const k = Math.min(maxTicks, lines.length)
	if (k <= 1) return logs.length > 0 ? [logs] : []
	const chunks: string[] = []
	let from = 0
	for (let i = 0; i < k; i++) {
		const to = Math.round(((i + 1) * lines.length) / k)
		chunks.push(lines.slice(from, to).join(''))
		from = to
	}
	return chunks.filter((c) => c.length > 0)
}

export type SynthesisOptions = {
	/** Recorded wall-clock ms that maps to t=0 on the replay clock. */
	anchorMs: number
	/** Wall-clock ms that t=0 plays at (usually `Date.now()` when Play is hit). */
	nowMs: number
	/** All the run's jobs, so a flow job's module timings can be resolved. */
	jobsById?: Record<string, Job>
	/** Reveal everything at t=0 instead of streaming — for a job the viewer
	 * opens after it already finished. */
	collapse?: boolean
	/** Shared across a multi-job replay so the aggregate caps hold; a fresh
	 * budget is used when absent (single-job replays). */
	budget?: SynthesisBudget
}

/** Turn one completed job into the stream of updates a live watch of it would
 * have produced. Never mutates `job`. */
export function synthesizeJobStream(job: Job, opts: SynthesisOptions): RecordedJob {
	const { anchorMs, nowMs, jobsById = {}, collapse } = opts
	const budget = opts.budget ?? newBudget()
	const offset = nowMs - anchorMs

	const completed: any = clone(job)
	rebaseJob(completed, offset)

	if (collapse) {
		return {
			initial_job: completed as Job,
			events: [{ t: 0, data: { completed: true, job: completed } }]
		}
	}

	const startT = clampT((jobStartMs(job) ?? anchorMs) - anchorMs)
	const durMs = jobDurationMs(job)
	const endT = clampT(startT + durMs)

	const initial: any = clone(completed)
	initial.type = 'QueuedJob'
	initial.running = false
	initial.logs = ''
	for (const k of ['success', 'result', 'result_stream', 'duration_ms', 'completed_at']) {
		delete initial[k]
	}
	if (job.flow_status) {
		initial.flow_status = flowStatusAt(startT, job.flow_status as FlowStatus, jobsById, anchorMs)
		rebaseFlowStatus(initial.flow_status, offset)
	}

	const events: RecordedEvent[] = [{ t: startT, data: { running: true } }]

	if (job.flow_status) {
		const statusCost = Math.max(1, roughNodeCount(job.flow_status))
		const allowed = Math.min(
			MAX_STATUS_SNAPSHOTS,
			Math.floor(budget.snapshotNodes / statusCost),
			budget.events
		)
		for (const t of statusBoundaries(job.flow_status as FlowStatus, jobsById, anchorMs, allowed)) {
			if (t <= startT || t >= endT) continue
			const fs = flowStatusAt(t, job.flow_status as FlowStatus, jobsById, anchorMs)
			rebaseFlowStatus(fs, offset)
			events.push({ t, data: { running: true, flow_status: fs } })
			budget.events--
			budget.snapshotNodes -= statusCost
		}
	}

	const logs = typeof (job as any).logs === 'string' ? ((job as any).logs as string) : ''
	if (logs.length > 0 && durMs > 0) {
		// At least one chunk even with the budget spent, so logs still appear —
		// a single event per job is bounded by the job count.
		const maxTicks = Math.max(
			1,
			Math.min(MAX_LOG_TICKS, Math.ceil(durMs / LOG_TICK_MS), budget.events)
		)
		const chunks = chunkLogs(logs, maxTicks)
		budget.events -= chunks.length
		let logOffset = 0
		chunks.forEach((chunk, i) => {
			logOffset += chunk.length
			// Strictly inside (startT, endT) so the last lines land just before
			// the completion event rather than racing it.
			const t = clampT(startT + (durMs * (i + 1)) / (chunks.length + 1))
			events.push({ t, data: { new_logs: chunk, log_offset: logOffset } })
		})
	}

	events.push({ t: endT, data: { completed: true, job: completed } })
	events.sort((a, b) => a.t - b.t)
	return { initial_job: initial as Job, events }
}

/** Replay stream for one standalone job (script run, pipeline node), anchored
 * on its own start so its events play from the moment the replay begins. */
export function synthesizeSingleJobReplay(
	job: Job,
	opts?: { collapse?: boolean; nowMs?: number }
): RecordedJob {
	return synthesizeJobStream(job, {
		anchorMs: jobStartMs(job) ?? 0,
		nowMs: opts?.nowMs ?? Date.now(),
		collapse: opts?.collapse
	})
}

/** The sub-job ids a flow job's status references directly. */
function directSubIds(job: Job | undefined): string[] {
	const fs: any = job?.flow_status
	if (!fs) return []
	const ids: string[] = []
	for (const mod of [...asArray<any>(fs.modules), fs.failure_module, fs.preprocessor_module]) {
		if (!mod || typeof mod !== 'object') continue
		if (typeof mod.job === 'string') ids.push(mod.job)
		for (const j of asArray<unknown>(mod.flow_jobs)) {
			if (typeof j === 'string') ids.push(j)
		}
	}
	return ids
}

/** Build the full replay for a recorded flow run: one stream per job, all on
 * the same clock (zero = root start), with every flow job's completion pushed
 * past its descendants' events — recorded durations can tie to the millisecond,
 * and a parent that "finishes" before its children breaks the animation. */
export function synthesizeFlowReplay(
	jobs: Record<string, Job>,
	rootJobId: string,
	nowMs: number = Date.now()
): ActiveReplayData {
	const root = jobs[rootJobId]
	const anchorMs =
		jobStartMs(root) ??
		Object.values(jobs)
			.map(jobStartMs)
			.filter((t): t is number => t !== undefined)
			.sort((a, b) => a - b)[0] ??
		0
	const budget = newBudget()
	const streams: Record<string, RecordedJob> = {}
	// Root first so the graph-driving status snapshots get first claim on the
	// budget before sub-job log ticks consume it.
	const ids = [rootJobId, ...Object.keys(jobs).filter((id) => id !== rootJobId)]
	for (const id of ids) {
		if (!jobs[id]) continue
		streams[id] = synthesizeJobStream(jobs[id], { anchorMs, nowMs, jobsById: jobs, budget })
	}
	// Bottom-up: bump each flow job's completed event past its descendants',
	// then the root's past everything (orphaned streams included).
	const visited = new Set<string>()
	const finalizeCompletion = (id: string): number => {
		const stream = streams[id]
		if (!stream) return 0
		const maxOwnT = () => stream.events.reduce((m, e) => Math.max(m, e.t), 0)
		if (visited.has(id)) return maxOwnT()
		visited.add(id)
		let childMax = 0
		for (const sub of directSubIds(jobs[id])) {
			if (sub !== id) childMax = Math.max(childMax, finalizeCompletion(sub))
		}
		const completedEvent = stream.events.find((e) => e.data.completed)
		if (completedEvent && childMax > 0 && completedEvent.t <= childMax) {
			completedEvent.t = clampT(childMax + 50)
			stream.events.sort((a, b) => a.t - b.t)
		}
		return maxOwnT()
	}
	finalizeCompletion(rootJobId)
	let maxT = 0
	for (const [id, stream] of Object.entries(streams)) {
		if (id === rootJobId) continue
		for (const e of stream.events) maxT = Math.max(maxT, e.t)
	}
	const rootEvents = streams[rootJobId]?.events
	const completedEvent = rootEvents?.find((e) => e.data.completed)
	if (completedEvent && completedEvent.t <= maxT) {
		completedEvent.t = clampT(maxT + 50)
		rootEvents!.sort((a, b) => a.t - b.t)
	}
	return { jobs: streams }
}
