let reqs: Record<string, any> = {}

function doRequest(type: string, o: object, extra?: object) {
	return new Promise((resolve, reject) => {
		const reqId = Math.random().toString(36)
		reqs[reqId] = { resolve, reject, ...extra }
		const req = { ...o, type, reqId }
		parent.postMessage(req, '*')
	})
}

export const backend = new Proxy(
	{},
	{
		get(_, runnable_id: string) {
			return (v: any) => {
				return doRequest('backend', { runnable_id, v })
			}
		}
	}
)

export const backendAsync = new Proxy(
	{},
	{
		get(_, runnable_id: string) {
			return (v: any) => {
				return doRequest('backendAsync', { runnable_id, v })
			}
		}
	}
)

export function waitJob(jobId: string) {
	return doRequest('waitJob', { jobId })
}

export function getJob(jobId: string) {
	return doRequest('getJob', { jobId })
}

/**
 * Stream job results using SSE. Calls onUpdate for each stream update,
 * and resolves with the final result when the job completes.
 * @param jobId - The job ID to stream
 * @param onUpdate - Callback for stream updates with new_result_stream data
 * @returns Promise that resolves with the final job result
 */
export function streamJob(
	jobId: string,
	onUpdate?: (data: { new_result_stream?: string; stream_offset?: number }) => void
): Promise<any> {
	return doRequest('streamJob', { jobId }, { onUpdate })
}

window.addEventListener('message', (e) => {
	if (e.data.type === 'streamJobUpdate') {
		// Handle streaming update
		let job = reqs[e.data.reqId]
		if (job && job.onUpdate) {
			job.onUpdate({
				new_result_stream: e.data.new_result_stream,
				stream_offset: e.data.stream_offset
			})
		}
	} else if (e.data.type === 'streamJobRes') {
		// Handle stream completion
		let job = reqs[e.data.reqId]
		if (job) {
			if (e.data.error) {
				job.reject(new Error(e.data.result?.stack ?? e.data.result?.message ?? 'Stream error'))
			} else {
				job.resolve(e.data.result)
			}
			delete reqs[e.data.reqId]
		}
	} else if (
		e.data.type === 'backendRes' ||
		e.data.type === 'backendAsyncRes' ||
		e.data.type === 'waitJobRes' ||
		e.data.type === 'getJobRes'
	) {
		console.log('Message from parent backend', e.data)
		let job = reqs[e.data.reqId]
		if (job) {
			const result = e.data.result
			if (e.data.error) {
				job.reject(new Error(result.stack ?? result.message))
			} else {
				job.resolve(result)
			}
			delete reqs[e.data.reqId]
		} else {
			console.error('No job found for', e.data.reqId)
		}
	}
})
