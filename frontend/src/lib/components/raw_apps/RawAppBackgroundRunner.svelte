<script lang="ts">
	import { executeRunnable } from '../apps/components/helpers/executeRunnable'
	import { userStore } from '$lib/stores'
	import { waitJob } from '../waitJob'
	import type { JobById } from '../apps/types'
	import { JobService } from '$lib/gen'
	import type { Runnable } from './rawAppPolicy'
	import { undefinedIfEmpty } from '$lib/utils'

	interface Props {
		iframe: HTMLIFrameElement | undefined
		path: string
		runnables: Record<string, Runnable>
		jobs?: string[]
		jobsById?: Record<string, JobById>
		editor: boolean
		workspace: string
	}

	let {
		iframe,
		path,
		runnables,
		jobs = $bindable([]),
		jobsById = $bindable({}),
		editor,
		workspace
	}: Props = $props()

	let listener = async (event) => {
		const data = event.data

		function respond(o: object) {
			iframe?.contentWindow?.postMessage({ type: data.type + 'Res', ...o, reqId: data.reqId }, '*')
		}
		async function respondWithResult(uuid: string) {
			let error = false
			let result
			try {
				result = await waitJob(uuid, workspace)
			} catch (e) {
				error = true
				console.log('e', e)
				result = e
			}

			if (event.data.type == 'backend' || event.data.type == 'waitJob') {
				respond({ result, error })
			}
			if (editor) {
				try {
					let jobInfo = await JobService.getCompletedJobTiming({ workspace, id: uuid })
					if (jobInfo.started_at) {
						jobsById[uuid] = {
							...(jobsById[uuid] ?? {}),
							created_at: new Date(jobInfo.created_at).getTime(),
							started_at: new Date(jobInfo.started_at).getTime(),
							duration_ms: jobInfo.duration_ms
						}
					}
				} catch (e) {
					console.error('Error getting job info', e)
				}
			}
			return result
		}
		if (event.data.type == 'backend' || event.data.type == 'backendAsync') {
			const runnable_id = data.runnable_id
			let runnable = runnables[runnable_id]
			if (runnable) {
				// Build args, converting ctx fields to $ctx:property format
				const args = { ...(data.v ?? {}) }
				for (const [key, field] of Object.entries(runnable?.fields ?? {})) {
					if (field?.type === 'ctx' && field?.ctx) {
						args[key] = `$ctx:${field.ctx}`
					}
				}
				const uuid = await executeRunnable(
					runnable,
					workspace,
					undefined,
					$userStore?.username,
					path,
					runnable_id,
					{
						component: runnable_id,
						args,
						force_viewer_allow_user_resources: editor
							? undefinedIfEmpty(
									Object.keys(runnable?.fields ?? {}).filter(
										(k) =>
											runnable?.fields?.[k]?.type == 'user' &&
											runnable?.fields?.[k]?.allowUserResources
									)
								)
							: undefined,
						force_viewer_one_of_fields: undefined,
						force_viewer_static_fields: editor
							? Object.fromEntries(
									Object.entries(runnable?.fields ?? {})
										.filter(([k, v]) => v.type == 'static')
										.map(([k, v]) => [k, v?.['value']])
								)
							: undefined
					},
					undefined
				)
				let job: JobById = { component: runnable_id, created_at: Date.now(), job: uuid }
				if (event.data.type == 'backendAsync') {
					let result = uuid
					respond({ result })
				}
				if (editor) {
					jobsById[uuid] = job
					jobs = [...jobs, uuid]
				}

				const result = await respondWithResult(uuid)

				if (editor) {
					job.result = result
				}
			} else {
				console.error('No runnable found for', runnable_id)
			}
		} else if (event.data.type == 'waitJob') {
			await respondWithResult(data.jobId)
		} else if (event.data.type == 'getJob') {
			const job = await JobService.getJob({ workspace, id: data.jobId })
			respond({ result: job })
		} else if (event.data.type == 'streamJob') {
			// Stream job results using SSE
			const jobId = data.jobId
			const reqId = data.reqId
			const params = new URLSearchParams()
			params.set('fast', 'true')
			params.set('only_result', 'true')

			const sseUrl = `/api/w/${workspace}/jobs_u/getupdate_sse/${jobId}?${params.toString()}`
			const eventSource = new EventSource(sseUrl)

			eventSource.onmessage = (sseEvent) => {
				try {
					const update = JSON.parse(sseEvent.data)
					const type = update.type

					if (type === 'ping' || type === 'timeout') {
						if (type === 'timeout') {
							eventSource.close()
						}
						return
					}

					if (type === 'error') {
						eventSource.close()
						iframe?.contentWindow?.postMessage(
							{
								type: 'streamJobRes',
								reqId,
								error: true,
								result: { message: update.error || 'SSE error' }
							},
							'*'
						)
						return
					}

					if (type === 'not_found') {
						eventSource.close()
						iframe?.contentWindow?.postMessage(
							{
								type: 'streamJobRes',
								reqId,
								error: true,
								result: { message: 'Job not found' }
							},
							'*'
						)
						return
					}

					// Send stream update if there's new stream data
					if (update.new_result_stream !== undefined) {
						iframe?.contentWindow?.postMessage(
							{
								type: 'streamJobUpdate',
								reqId,
								new_result_stream: update.new_result_stream,
								stream_offset: update.stream_offset
							},
							'*'
						)
					}

					// Check if job is completed
					if (update.completed) {
						eventSource.close()
						iframe?.contentWindow?.postMessage(
							{
								type: 'streamJobRes',
								reqId,
								error: false,
								result: update.only_result
							},
							'*'
						)
					}
				} catch (parseErr) {
					console.warn('Failed to parse SSE data:', parseErr)
				}
			}

			eventSource.onerror = (error) => {
				console.warn('SSE stream error:', error)
				eventSource.close()
				iframe?.contentWindow?.postMessage(
					{
						type: 'streamJobRes',
						reqId,
						error: true,
						result: { message: 'SSE connection error' }
					},
					'*'
				)
			}
		}
	}
</script>

<svelte:window onmessage={listener} />
