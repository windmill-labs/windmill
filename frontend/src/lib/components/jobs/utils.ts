import { JobService, type RunScriptPreviewData } from '$lib/gen'

export async function runPreviewJobAndPollResult(
	data: RunScriptPreviewData,
	{ maxRetries = 7 }: { maxRetries?: number } = {}
): Promise<unknown> {
	const uuid = await JobService.runScriptPreview(data)

	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 500 * (attempts || 0.75)))
			const job = await JobService.getCompletedJob({
				id: uuid,
				workspace: data.workspace
			})
			if (job.success) {
				return job.result
			} else {
				attempts = maxRetries
				let errorMsg: string | undefined = (job.result as any).error.message
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
	console.error('Could not get job result')
}
