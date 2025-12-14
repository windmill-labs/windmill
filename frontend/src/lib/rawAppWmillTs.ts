let reqs: Record<string, any> = {}

function doRequest(type: string, o: object) {
	return new Promise((resolve, reject) => {
		const reqId = Math.random().toString(36)
		reqs[reqId] = { resolve, reject }
		parent.postMessage({ ...o, type, reqId }, '*')
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

window.addEventListener('message', (e) => {
	if (e.data.type == 'backendRes' || e.data.type == 'backendAsyncRes') {
		console.log('Message from parent backend', e.data)
		let job = reqs[e.data.reqId]
		if (job) {
			const result = e.data.result
			if (e.data.error) {
				job.reject(new Error(result.stack ?? result.message))
			} else {
				job.resolve(result)
			}
		} else {
			console.error('No job found for', e.data.reqId)
		}
	}
})
