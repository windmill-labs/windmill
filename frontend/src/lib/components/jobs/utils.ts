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
	 * The job writes (insert/update/delete, DDL, arbitrary SQL). Such a job must
	 * never stay executable once its caller has handled it as failed, or a later
	 * pickup plus a retry applies it twice — so it is cancelled before the
	 * missing-worker error is reported, and the poll keeps waiting if the cancel
	 * is refused. Reads are abandoned queued instead, which leaves the autoscaler
	 * the backlog it scales up on.
	 */
	sideEffecting?: boolean
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
		sideEffecting = false
	}: RunScriptOptions = {}
): Promise<unknown> {
	let attempts = 0
	let polls = 0
	// `attempts` only advances on errors, so a queued job would poll forever. The
	// one case that never resolves on its own is a tag no worker serves, which
	// takes NO_WORKER_CONFIRMATIONS consecutive empty lookups to establish — a
	// worker group booting reads like an unserved tag in any single one.
	let noWorkerProbeAt = Date.now() + NO_WORKER_FIRST_PROBE_MS
	let unservedProbes = 0
	// Set once a cancel has been *requested* for an unserved write, so the loop
	// can name the cause when the job turns out to have ended cancelled.
	let cancelRequestedForTag: string | undefined = undefined
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
				if (cancelRequestedForTag) {
					throw new NoWorkerForTagError(cancelRequestedForTag, true)
				}
				let errorMsg: string | undefined = (job?.result as any)?.error?.message
				if (typeof errorMsg !== 'string') errorMsg = undefined
				console.error('JOB FAILED', job.result)
				throw new Error(errorMsg ?? 'Job failed')
			} else if (failIfNoWorkerForTag && Date.now() >= noWorkerProbeAt) {
				const tag = await missingWorkerTagOfQueuedJob(workspace, uuid)
				noWorkerProbeAt = Date.now() + NO_WORKER_PROBE_INTERVAL_MS
				unservedProbes = tag ? unservedProbes + 1 : 0
				if (tag && unservedProbes >= NO_WORKER_CONFIRMATIONS) {
					if (!sideEffecting) {
						// Only the wait is given up on — the job is left queued. Cancelling a
						// read would remove the very backlog the autoscaler scales up on, so a
						// group coming back from zero (300s cooldown) would never recover.
						throw new NoWorkerForTagError(tag, false)
					}
					// A write must not stay executable once its caller has handled it as
					// failed. The cancel is only a request though: it answers 200 for a job
					// a worker claimed and completed in the meantime too, so the outcome is
					// read from the next poll — a write that actually landed still returns
					// its result above.
					await JobService.cancelQueuedJob({ workspace, id: uuid, requestBody: {} }).then(
						() => (cancelRequestedForTag = tag),
						(err) => console.warn('Could not cancel the unpickable job', err)
					)
					unservedProbes = 0
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
