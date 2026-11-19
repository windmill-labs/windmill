import { JobService, type Job } from '$lib/gen'

// A finished job's tree never changes, so its count is kept for the page's life; a running
// one is asked again on the next status update.
const finishedCounts = new Map<string, Promise<number>>()

/**
 * Error handlers that ran anywhere under a flow job: its own, and those of every subflow,
 * loop iteration and branch below it. Each iteration and branch is its own flow job carrying a
 * copy of the handler, so the walk visits them all.
 */
export function countErrorHandlersUnder(workspace: string, jobId: string): Promise<number> {
	const cached = finishedCounts.get(jobId)
	if (cached) return cached
	const count = JobService.getJob({ workspace, id: jobId, noLogs: true, noCode: true }).then(
		async (job) => {
			const n = await countIn(workspace, job)
			if (job.type !== 'CompletedJob') finishedCounts.delete(jobId)
			return n
		},
		(err) => {
			finishedCounts.delete(jobId)
			throw err
		}
	)
	finishedCounts.set(jobId, count)
	return count
}

async function countIn(workspace: string, job: Job): Promise<number> {
	const status = job.flow_status
	if (!status) return 0
	const own = status.failure_module?.job ? 1 : 0
	const types = new Map(job.raw_flow?.modules.map((m) => [m.id, m.value.type]) ?? [])
	const children: string[] = []
	for (const mod of status.modules) {
		if (mod.flow_jobs?.length) {
			children.push(...mod.flow_jobs)
		} else if (mod.job) {
			// A script step has no flow below it; only these run a flow job of their own.
			const type = mod.id ? types.get(mod.id) : undefined
			if (type === 'flow' || type === 'branchone') children.push(mod.job)
		}
	}
	const counts = await Promise.all(children.map((id) => countErrorHandlersUnder(workspace, id)))
	return counts.reduce((a, b) => a + b, own)
}
