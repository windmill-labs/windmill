import { JobService, type RunScriptByPathData, type RunScriptPreviewData } from '$lib/gen'

function isRunScriptByPathData(
	arg: RunScriptPreviewData | RunScriptByPathData
): arg is RunScriptByPathData {
	return (arg as RunScriptByPathData).path !== undefined
}

type RunScriptOptions = {
	maxRetries?: number
	withJobData?: boolean
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
	{ maxRetries = 7, withJobData }: RunScriptOptions = {}
): Promise<unknown> {
	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 500 * (attempts || 0.75)))
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
			}
		} catch (e) {
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
