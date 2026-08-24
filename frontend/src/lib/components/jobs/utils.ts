import { JobService, type RunScriptByPathData, type RunScriptPreviewData } from '$lib/gen'
import {
	missingWorkerTagOfQueuedJob,
	NoWorkerForTagError,
	NO_WORKER_CONFIRMATIONS,
	NO_WORKER_FIRST_PROBE_MS,
	NO_WORKER_PROBE_INTERVAL_MS
} from './missingWorker'

function isRunScriptByPathData(
	arg: RunScriptPreviewData | RunScriptByPathData
): arg is RunScriptByPathData {
	return (arg as RunScriptByPathData).path !== undefined
}

type RunScriptOptions = {
	maxRetries?: number
	withJobData?: boolean
	/** Set to false to keep polling a job that no worker can pick up. */
	failIfNoWorkerForTag?: boolean
	/**
	 * The job writes (insert/update/delete, DDL, arbitrary SQL). Such a job is
	 * never given up on: reporting it as failed while it stays executable invites
	 * a duplicate retry, and cancelling it first is not something the client can
	 * do atomically: a worker can claim it between the tag probe and the cancel,
	 * and a soft-cancelled statement may already have committed. `onNoWorkerForTag`
	 * is what tells the user why it is waiting.
	 */
	sideEffecting?: boolean
	/** Called once when the job has stayed queued on a tag no worker serves. */
	onNoWorkerForTag?: (tag: string) => void
}

/**
 * @function runScript
 * @param {RunScriptPreviewData | RunScriptByPathData} data - Data for running the script.
 * @returns {Promise<string>} A UUID representing the running script.
 *
 * @example
 * const uuid = await runScript(data)
 */
export async function runScript(data: RunScriptPreviewData | RunScriptByPathData) {
	const uuid = (
		isRunScriptByPathData(data)
			? await JobService.runScriptByPath(data)
			: await JobService.runScriptPreview(data)
	) as string

	return uuid
}

/** Tight at first so a quick job feels instant, then slower: a schema
 * introspection or a DDL migration can run for minutes, and a fixed sub-second
 * tick would cost hundreds of round-trips for it. */
function pollDelayMs(poll: number): number {
	if (poll < 4) return 375
	if (poll < 12) return 750
	return 2000
}

/**
 * @function pollJobResult
 * @description Polls a job result by UUID until success, failure, or max retries reached.
 * @param {string} uuid - Job UUID.
 * @param {string} workspace - Workspace identifier.
 * @param {RunScriptOptions} [options] - Optional settings like retries and job data inclusion.
 * @returns {Promise<unknown>} Final job result or throws error if it fails.
 *
 * @example
 * const result = await pollJobResult(uuid, 'my-workspace', { maxRetries: 5, withJobData: true });
 */
export async function pollJobResult(
	uuid: string,
	workspace: string,
	{
		maxRetries = 7,
		withJobData,
		failIfNoWorkerForTag = true,
		sideEffecting = false,
		onNoWorkerForTag
	}: RunScriptOptions = {}
): Promise<unknown> {
	let attempts = 0
	let polls = 0
	// `attempts` only advances on errors, so a queued job would poll forever. The
	// one case that never resolves on its own is a tag no worker serves, which
	// takes NO_WORKER_CONFIRMATIONS consecutive empty lookups to establish, since
	// a worker group booting reads like an unserved tag in any single one.
	let noWorkerProbeAt = Date.now() + NO_WORKER_FIRST_PROBE_MS
	let unservedProbes = 0
	let reportedNoWorker = false
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) =>
				setTimeout(resolve, attempts ? 500 * attempts : pollDelayMs(polls++))
			)
			const job = await JobService.getCompletedJobResultMaybe({
				id: uuid,
				workspace
			})
			if (job.success) {
				if (withJobData) {
					return { job: { id: uuid }, result: job.result }
				} else {
					return job.result as any
				}
			} else if (job.completed) {
				attempts = maxRetries
				let errorMsg: string | undefined = (job?.result as any)?.error?.message
				if (typeof errorMsg !== 'string') errorMsg = undefined
				console.error('JOB FAILED', job.result)
				throw new Error(errorMsg ?? 'Job failed')
			} else if (failIfNoWorkerForTag && Date.now() >= noWorkerProbeAt) {
				const tag = await missingWorkerTagOfQueuedJob(workspace, uuid)
				noWorkerProbeAt = Date.now() + NO_WORKER_PROBE_INTERVAL_MS
				unservedProbes = tag ? unservedProbes + 1 : 0
				if (tag && unservedProbes >= NO_WORKER_CONFIRMATIONS) {
					if (!reportedNoWorker) {
						reportedNoWorker = true
						onNoWorkerForTag?.(tag)
					}
					// Reads give up the wait but leave the job queued: cancelling one would
					// remove the very backlog the autoscaler scales up on, so a group coming
					// back from zero (300s cooldown) would never recover. Writes keep
					// waiting instead (see `sideEffecting`).
					if (!sideEffecting) throw new NoWorkerForTagError(tag)
				}
			}
		} catch (e) {
			if (e instanceof NoWorkerForTagError) {
				throw e
			}
			if (attempts == maxRetries) {
				throw e
			}
			attempts++
		}
	}

	throw 'Could not get job result, should not get here'
}

export async function runScriptAndPollResult(
	data: RunScriptPreviewData | RunScriptByPathData,
	runScriptOptions?: RunScriptOptions
): Promise<unknown> {
	const uuid = await runScript(data)

	return await pollJobResult(uuid, data.workspace, runScriptOptions)
}
