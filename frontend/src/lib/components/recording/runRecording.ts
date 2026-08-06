/**
 * Builds recordings from completed runs. A recording is a pure function of a
 * finished job: the root job plus every sub-job (nested flows included),
 * fetched from the API with their logs and results. Nothing is captured while
 * the run is live — replay streams are synthesized from the jobs' timestamps
 * at play time (see `replayStream.ts`).
 */
import { JobService, type Job, type OpenFlow } from '$lib/gen'
import type { FlowRecording, ScriptRecording } from './types'

const UUID_RE = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi

/** Shorten every UUID in the serialized recording to a stable 8-char form, so
 * downloaded files don't leak full job ids and stay diff-friendly. The same
 * UUID always maps to the same short form, keeping job references consistent. */
export function truncateUuids(json: string): string {
	const map = new Map<string, string>()
	let counter = 0
	return json.replace(UUID_RE, (uuid) => {
		const key = uuid.toLowerCase()
		let short = map.get(key)
		if (!short) {
			short = counter === 0 ? key.slice(-8) : key.slice(-8) + '_' + counter
			counter++
			map.set(key, short)
		}
		return short
	})
}

/** Serialize `recording` and hand it to the browser as a JSON download. */
export function downloadRecordingJson(recording: unknown, filenamePrefix: string) {
	const blob = new Blob([truncateUuids(JSON.stringify(recording, null, 2))], {
		type: 'application/json'
	})
	const url = URL.createObjectURL(blob)
	const a = document.createElement('a')
	a.href = url
	a.download = `${filenamePrefix}-${Date.now()}.json`
	a.click()
	URL.revokeObjectURL(url)
}

/** The sub-job ids a flow job's status references directly (one level). */
function directSubJobIds(flowStatus: Job['flow_status']): string[] {
	if (!flowStatus) return []
	const ids: string[] = []
	for (const mod of flowStatus.modules ?? []) {
		if (mod.job) ids.push(mod.job)
		if (mod.flow_jobs) ids.push(...mod.flow_jobs)
	}
	if (flowStatus.failure_module?.job) ids.push(flowStatus.failure_module.job)
	if (flowStatus.preprocessor_module?.job) ids.push(flowStatus.preprocessor_module.job)
	return ids
}

/** Recording size backstop: a pathological run (huge loops) stops collecting
 * here rather than fetching thousands of jobs. */
const MAX_RECORDING_FETCH_JOBS = 1000
/** Concurrent job fetches while collecting a run. */
const FETCH_CONCURRENCY = 20

/** Cap on one job's recorded logs — the replay loader refuses any value past
 * ~8MB of text, so an unbounded capture would record fine and then never play. */
const MAX_RECORDED_LOG_CHARS = 2 * 1024 * 1024

/** Fetch a job with its complete logs: `getJob` returns only the tail of the
 * log column (and only the residual chunk once logs are offloaded to object
 * storage), so the full text has to come from the logs endpoint. Best-effort —
 * on failure the job keeps whatever `getJob` returned. */
export async function fetchJobWithFullLogs(workspace: string, id: string): Promise<Job> {
	const [job, logs] = await Promise.all([
		JobService.getJob({ workspace, id }),
		JobService.getJobLogs({ workspace, id }).catch(() => undefined)
	])
	if (typeof logs === 'string' && logs.length >= ((job as any).logs?.length ?? 0)) {
		;(job as any).logs =
			logs.length > MAX_RECORDED_LOG_CHARS
				? '[logs truncated for recording]\n…' + logs.slice(-MAX_RECORDED_LOG_CHARS)
				: logs
	}
	return job
}

/** Fetch `rootJobId` and, recursively, every sub-job its flow status
 * references. Fetch failures are tolerated — a job that can't be fetched
 * simply replays with less detail. */
async function fetchRunJobs(workspace: string, rootJobId: string): Promise<Record<string, Job>> {
	const jobs: Record<string, Job> = {}
	let frontier = [rootJobId]
	while (frontier.length > 0 && Object.keys(jobs).length < MAX_RECORDING_FETCH_JOBS) {
		frontier = [...new Set(frontier)]
			.filter((id) => !jobs[id])
			.slice(0, MAX_RECORDING_FETCH_JOBS - Object.keys(jobs).length)
		// Bounded fan-out: each job costs two requests, and a wide loop's
		// frontier would otherwise put hundreds in flight at once.
		const fetched: (Job | undefined)[] = []
		for (let i = 0; i < frontier.length; i += FETCH_CONCURRENCY) {
			fetched.push(
				...(await Promise.all(
					frontier.slice(i, i + FETCH_CONCURRENCY).map(async (id) => {
						try {
							return await fetchJobWithFullLogs(workspace, id)
						} catch (e) {
							console.warn('[recording] failed to fetch job', id, e)
							return undefined
						}
					})
				))
			)
		}
		const next: string[] = []
		for (const job of fetched) {
			if (!job) continue
			jobs[job.id] = job
			next.push(...directSubJobIds(job.flow_status))
		}
		frontier = next
	}
	return jobs
}

/** Build a flow recording from a completed run. */
export async function buildFlowRecording(
	workspace: string,
	rootJobId: string,
	flowPath: string,
	flow?: OpenFlow
): Promise<FlowRecording> {
	const jobs = await fetchRunJobs(workspace, rootJobId)
	const root = jobs[rootJobId]
	if (!root) {
		throw new Error('Could not fetch the run this recording is based on')
	}
	return {
		version: 2,
		type: 'flow',
		recorded_at: new Date().toISOString(),
		flow_path: flowPath,
		total_duration_ms: (root as any).duration_ms ?? 0,
		root_job_id: rootJobId,
		jobs,
		// JSON round-trip to strip non-serializable properties (event handlers, …)
		flow: flow ? (JSON.parse(JSON.stringify(flow)) as OpenFlow) : undefined
	}
}

/** Build a script recording from a completed test run. Code/schema are passed
 * in as they were at run time — a preview job doesn't carry the schema and its
 * code may since have been edited. `args` defaults to the job's own. */
export async function buildScriptRecording(
	workspace: string,
	jobId: string,
	meta: {
		scriptPath: string
		code: string
		language: string
		args?: Record<string, any>
		schema?: Record<string, any>
	}
): Promise<ScriptRecording> {
	const job = await fetchJobWithFullLogs(workspace, jobId)
	return {
		version: 2,
		type: 'script',
		recorded_at: new Date().toISOString(),
		script_path: meta.scriptPath,
		total_duration_ms: (job as any).duration_ms ?? 0,
		code: meta.code,
		language: meta.language,
		args: JSON.parse(JSON.stringify(meta.args ?? job.args ?? {})),
		schema: meta.schema ? JSON.parse(JSON.stringify(meta.schema)) : undefined,
		job
	}
}
