import { JobService, type RunScriptPreviewData } from '$lib/gen'

export async function runPreviewJobAndPollResult(
	data: RunScriptPreviewData,
	{ maxRetries = 7, withJobData }: { maxRetries?: number; withJobData?: boolean } = {}
): Promise<unknown> {
	const uuid = await JobService.runScriptPreview(data)

	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 500 * (attempts || 0.75)))
			const job = await JobService.getCompletedJobResultMaybe({
				id: uuid,
				workspace: data.workspace
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
