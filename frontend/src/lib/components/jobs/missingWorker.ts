import { JobService, WorkerService } from '$lib/gen'

/**
 * A queued job whose tag no running worker serves is never picked up: without
 * this the UI polls until the server-side `run_wait_result` timeout (10min by
 * default) or, for the client-side pollers, forever.
 *
 * The most common cause is a language that defaults to a native tag
 * (`postgresql`, `mysql`, `bigquery`, …) on an instance whose worker groups
 * only declare the default tags.
 */
export class NoWorkerForTagError extends Error {
	tag: string

	constructor(tag: string) {
		super(
			`No worker is currently listening to the tag "${tag}", so this job was not picked up ` +
				`and has been cancelled. Add "${tag}" to the worker tags of one of your worker groups ` +
				`(Workers page), or run a worker that serves it.`
		)
		this.name = 'NoWorkerForTagError'
		this.tag = tag
	}
}

/** How long a job may sit un-started before we look for a worker serving its tag. */
export const NO_WORKER_GRACE_MS = 8_000
/** How long to wait before repeating the lookup while the job stays queued. */
export const NO_WORKER_RECHECK_MS = 8_000

/**
 * Whether any worker pinged in the last minute declares `tag`. Unknown answers
 * (the endpoint returns an empty map when tags are sensitive and the caller may
 * not see them) count as "yes", so an opaque instance never gets a wrong
 * diagnosis.
 */
export async function hasWorkerForTag(workspace: string, tag: string): Promise<boolean> {
	const existing = await WorkerService.existsWorkersWithTags({ workspace, tags: tag })
	return existing[tag] !== false
}

/**
 * The tag of `jobId` when it is still queued and no worker serves it, else
 * undefined. Never throws: a failed lookup means "can't tell", and the caller
 * keeps waiting rather than reporting a cause it did not establish.
 */
export async function missingWorkerTagOfQueuedJob(
	workspace: string,
	jobId: string
): Promise<string | undefined> {
	try {
		const job = await JobService.getJob({ workspace, id: jobId, noCode: true, noLogs: true })
		if (job.type !== 'QueuedJob' || job.running || !job.tag) return undefined
		return (await hasWorkerForTag(workspace, job.tag)) ? undefined : job.tag
	} catch (err) {
		console.warn('Could not determine whether a worker serves the job tag', err)
		return undefined
	}
}
