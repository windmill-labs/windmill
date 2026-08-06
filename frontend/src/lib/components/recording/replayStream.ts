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
 */
import type { FlowStatus, FlowStatusModule, Job } from '$lib/gen'
import type { ActiveReplayData, RecordedEvent, RecordedJob } from './types'

/** Target pacing of synthesized log chunks. */
const LOG_TICK_MS = 200
/** Cap on log events per job — each becomes a `setTimeout` scheduled up front. */
const MAX_LOG_TICKS = 300
/** Cap on flow-status snapshots per flow job, same reason. */
const MAX_STATUS_SNAPSHOTS = 500
/** Recorded timestamps are caller-controlled (uploaded files), so no event may
 * schedule beyond this horizon. */
const MAX_REPLAY_MS = 6 * 60 * 60 * 1000

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

function rebaseFlowStatus(fs: any, offset: number) {
	if (!fs) return
	const mods = [...(fs.modules ?? []), fs.failure_module, fs.preprocessor_module]
	for (const mod of mods) {
		const durations = mod?.flow_jobs_duration
		if (durations?.started_at) {
			durations.started_at = durations.started_at.map((d: string) => offsetDate(d, offset) ?? d)
		}
	}
}

/** Shift a (cloned) job's absolute timestamps by `offset` ms so elapsed-time
 * displays anchored on `Date.now()` read correctly during replay. */
function rebaseJob(job: any, offset: number) {
	for (const k of ['started_at', 'created_at', 'completed_at', 'scheduled_for']) {
		if (job[k]) job[k] = offsetDate(job[k], offset)
	}
	rebaseFlowStatus(job.flow_status, offset)
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v))

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
	const fjd = mod.flow_jobs_duration
	if (fjd?.started_at?.length) {
		fjd.started_at.forEach((s, i) => {
			const st = parseMs(s)
			if (st === undefined) return
			const d = fjd.duration_ms?.[i]
			starts.push(st - anchorMs)
			ends.push(st - anchorMs + (typeof d === 'number' && Number.isFinite(d) && d > 0 ? d : 0))
		})
	} else if (mod.flow_jobs?.length) {
		for (const jid of mod.flow_jobs) {
			const j = jobsById[jid]
			const st = jobStartMs(j)
			if (j === undefined || st === undefined) continue
			starts.push(st - anchorMs)
			ends.push(st - anchorMs + jobDurationMs(j))
		}
	} else if (mod.job && jobsById[mod.job]) {
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

/** The module as it looked at time `T`: untouched final state once its window
 * has passed, a bare waiting marker before it, and in between an in-progress
 * variant with loop iterations trimmed to those already started. */
function moduleAt(
	T: number,
	mod: FlowStatusModule,
	jobsById: Record<string, Job>,
	anchorMs: number
): FlowStatusModule {
	const span = moduleSpan(mod, jobsById, anchorMs)
	if (!span || T >= span.end) return mod
	if (T < span.start) {
		return { id: mod.id, type: 'WaitingForPriorSteps' }
	}
	const started = span.iterStarts.filter((s) => s <= T).length
	const m: FlowStatusModule = { ...mod, type: 'InProgress' }
	if (mod.flow_jobs) {
		m.flow_jobs = mod.flow_jobs.slice(0, started)
		if (mod.flow_jobs_success) m.flow_jobs_success = mod.flow_jobs_success.slice(0, started)
		if (mod.flow_jobs_duration) {
			m.flow_jobs_duration = {
				started_at: mod.flow_jobs_duration.started_at?.slice(0, started),
				duration_ms: mod.flow_jobs_duration.duration_ms?.slice(0, started)
			}
		}
		if (mod.iterator) {
			m.iterator = { ...mod.iterator, index: Math.max(0, started - 1) }
		}
	}
	return m
}

/** The whole flow status as it looked at time `T`, derived from the completed
 * status plus the sub-jobs' timings. */
function flowStatusAt(
	T: number,
	final: FlowStatus,
	jobsById: Record<string, Job>,
	anchorMs: number
): FlowStatus {
	const modules = (final.modules ?? []).map((mod) => moduleAt(T, mod, jobsById, anchorMs))
	let step = 0
	for (const mod of final.modules ?? []) {
		const span = moduleSpan(mod, jobsById, anchorMs)
		if (span && T >= span.end) step++
		else break
	}
	const fs: FlowStatus = { ...final, step, modules }
	if (final.failure_module) {
		fs.failure_module = moduleAt(T, final.failure_module, jobsById, anchorMs) as any
	}
	if (final.preprocessor_module) {
		fs.preprocessor_module = moduleAt(T, final.preprocessor_module, jobsById, anchorMs)
	}
	return fs
}

/** The times at which some module's shown state changes — each one gets a
 * flow-status snapshot event. */
function statusBoundaries(
	final: FlowStatus,
	jobsById: Record<string, Job>,
	anchorMs: number
): number[] {
	const ts = new Set<number>()
	const mods = [...(final.modules ?? []), final.failure_module, final.preprocessor_module]
	for (const mod of mods) {
		if (!mod) continue
		const span = moduleSpan(mod, jobsById, anchorMs)
		if (!span) continue
		ts.add(span.start)
		ts.add(span.end)
		for (const s of span.iterStarts) ts.add(s)
	}
	let sorted = [...ts].sort((a, b) => a - b)
	if (sorted.length > MAX_STATUS_SNAPSHOTS) {
		const stride = sorted.length / MAX_STATUS_SNAPSHOTS
		sorted = Array.from({ length: MAX_STATUS_SNAPSHOTS }, (_, i) => sorted[Math.floor(i * stride)])
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
}

/** Turn one completed job into the stream of updates a live watch of it would
 * have produced. */
export function synthesizeJobStream(job: Job, opts: SynthesisOptions): RecordedJob {
	const { anchorMs, nowMs, jobsById = {}, collapse } = opts
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
		for (const t of statusBoundaries(job.flow_status as FlowStatus, jobsById, anchorMs)) {
			if (t <= startT || t >= endT) continue
			const fs = flowStatusAt(t, job.flow_status as FlowStatus, jobsById, anchorMs)
			rebaseFlowStatus(fs, offset)
			events.push({ t, data: { running: true, flow_status: fs } })
		}
	}

	const logs = typeof (job as any).logs === 'string' ? ((job as any).logs as string) : ''
	if (logs.length > 0 && durMs > 0) {
		const maxTicks = Math.min(MAX_LOG_TICKS, Math.max(1, Math.ceil(durMs / LOG_TICK_MS)))
		const chunks = chunkLogs(logs, maxTicks)
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

/** Build the full replay for a recorded flow run: one stream per job, all on
 * the same clock (zero = root start), with the root's completion guaranteed to
 * land after every sub-job event so the flow never "finishes" mid-animation. */
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
	const streams: Record<string, RecordedJob> = {}
	let maxT = 0
	for (const [id, job] of Object.entries(jobs)) {
		const stream = synthesizeJobStream(job, { anchorMs, nowMs, jobsById: jobs })
		streams[id] = stream
		if (id !== rootJobId) {
			for (const e of stream.events) maxT = Math.max(maxT, e.t)
		}
	}
	const rootEvents = streams[rootJobId]?.events
	const completedEvent = rootEvents?.find((e) => e.data.completed)
	if (completedEvent && completedEvent.t <= maxT) {
		completedEvent.t = clampT(maxT + 50)
		rootEvents!.sort((a, b) => a.t - b.t)
	}
	return { jobs: streams }
}
