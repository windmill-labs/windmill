import { JobService, type RunScriptPreviewData } from '$lib/gen'

export async function runPreviewJobAndPollResult(data: RunScriptPreviewData): Promise<unknown> {
	const uuid = await JobService.runScriptPreview(data)

	const maxRetries = 7
	let attempts = 0
	while (attempts < maxRetries) {
		try {
			await new Promise((resolve) => setTimeout(resolve, 500 * (attempts || 0.75)))
			const result = await JobService.getCompletedJob({
				id: uuid,
				workspace: data.workspace
			})
			if (result.success) {
				return result.result
			}
			attempts++
		} catch (e) {
			attempts++
		}
	}
}
