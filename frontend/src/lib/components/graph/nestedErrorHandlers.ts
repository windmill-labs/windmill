import { JobService, type Job } from '$lib/gen'
import { getAllModules } from '../flows/flowExplorer'

// A finished job's tree never changes, so its count is kept for the page's life; a running
// one, or one whose walk failed, is asked again next time.
const finishedCounts = new Map<string, Promise<number>>()

// A run graph can hold many loops and subflows: their walks share these slots so opening a run
// never sends more than a few job requests at once.
const MAX_REQUESTS = 4
let inFlight = 0
const waiting: (() => void)[] = []

async function getJob(workspace: string, id: string): Promise<Job> {
	if (inFlight >= MAX_REQUESTS) await new Promise<void>((resolve) => waiting.push(resolve))
	inFlight++
	try {
		return await JobService.getJob({ workspace, id, noLogs: true, noCode: true })
	} finally {
		inFlight--
		waiting.shift()?.()
	}
}

/**
 * Error handlers that ran anywhere under a flow job: its own, and those of every subflow,
 * loop iteration and branch below it. Each iteration and branch is its own flow job carrying a
 * copy of the handler, so the walk visits them all, unless no handler can exist below.
 */
export function countErrorHandlersUnder(workspace: string, jobId: string): Promise<number> {
	const cached = finishedCounts.get(jobId)
	if (cached) return cached
	const count = getJob(workspace, jobId).then(async (job) => {
		const n = await countIn(workspace, job)
		if (job.type !== 'CompletedJob') forget(jobId, count)
		return n
	})
	count.catch(() => forget(jobId, count))
	finishedCounts.set(jobId, count)
	return count
}

function forget(jobId: string, count: Promise<number>) {
	if (finishedCounts.get(jobId) === count) finishedCounts.delete(jobId)
}

async function countIn(workspace: string, job: Job): Promise<number> {
	const status = job.flow_status
	if (!status) return 0
	const own = status.failure_module?.job ? 1 : 0
	const flow = job.raw_flow
	// Iterations and branches only carry this flow's handler, so without one, and without a
	// subflow of its own, nothing below can have run a handler.
	const modules = flow ? getAllModules(flow.modules) : []
	if (!flow?.failure_module && !modules.some((m) => m.value.type === 'flow')) return own
	const types = new Map(flow?.modules.map((m) => [m.id, m.value.type]) ?? [])
	const children: string[] = []
	for (const mod of status.modules) {
		if (mod.flow_jobs?.length) {
			// Failed iterations and branches only, as in `firstIterationWithHandler`.
			children.push(...mod.flow_jobs.filter((_, i) => mod.flow_jobs_success?.[i] === false))
		} else if (mod.job) {
			// A script step has no flow below it; only these run a flow job of their own.
			const type = mod.id ? types.get(mod.id) : undefined
			if (type === 'flow' || type === 'branchone') children.push(mod.job)
		}
	}
	const counts = await Promise.all(children.map((id) => countErrorHandlersUnder(workspace, id)))
	return counts.reduce((a, b) => a + b, own)
}

/**
 * The first failed iteration of a loop that ran an error handler. Only failed iterations are
 * asked: a loop of thousands of successful iterations would otherwise cost a request each. A
 * handler that recovered leaves its iteration a success, so it shows only once that iteration
 * is opened.
 */
export async function firstIterationWithHandler(
	workspace: string,
	iterations: string[],
	success: (boolean | undefined)[] | undefined
): Promise<number | undefined> {
	for (let i = 0; i < iterations.length; i++) {
		if (success?.[i] !== false) continue
		if ((await countErrorHandlersUnder(workspace, iterations[i])) > 0) return i
	}
	return undefined
}
