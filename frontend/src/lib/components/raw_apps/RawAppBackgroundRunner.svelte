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
		/**
		 * Restrict waitJob/getJob/streamJob to job ids launched by this app
		 * instance (WIN-2006): a SANDBOXED bundle must not read arbitrary
		 * workspace jobs through the credentialed bridge. Off for unsandboxed
		 * renders (the default, and editor preview) — there the bundle holds
		 * the same credential as the bridge, so gating adds nothing and would
		 * only break unsandboxed apps that poll persisted or runnable-returned
		 * job ids.
		 */
		gateJobIds?: boolean
		/**
		 * Additional trusted message source beyond the bundle iframe: the
		 * detached preview window opened from the editor ("open preview in a
		 * separate window"). Its app bundle posts runnable requests to
		 * `window.opener` (this window), so the bridge must accept its
		 * `event.source` and reply to it. A getter so it tracks the live handle
		 * without a reactive prop. Editor-only — the detached window runs the
		 * same unsandboxed bundle as the inline preview.
		 */
		extraSourceWindow?: () => Window | null | undefined
	}

	let {
		iframe,
		path,
		runnables,
		jobs = $bindable([]),
		jobsById = $bindable({}),
		editor,
		workspace,
		gateJobIds = true,
		extraSourceWindow
	}: Props = $props()

	// Job ids launched by this app instance — see `gateJobIds`.
	const launchedJobs = new Set<string>()

	let listener = async (event) => {
		// Only accept messages from the bundle iframe (opaque origin) or the
		// detached preview window we opened, so other frames/extensions can't
		// drive the runnable bridge (WIN-2006). Reject unconditionally until the
		// iframe is bound — never process a message from an unknown source.
		const detachedWindow = extraSourceWindow?.()
		const sourceWindow = event.source as Window | null
		if (!iframe || !sourceWindow) return
		if (sourceWindow !== iframe.contentWindow && sourceWindow !== detachedWindow) return

		const data = event.data

		// Reply to whichever window sent the request (inline iframe or the
		// detached preview), not a hardcoded target — otherwise the detached
		// window's calls would hang waiting for a response routed elsewhere.
		function respond(o: object) {
			sourceWindow?.postMessage({ type: data.type + 'Res', ...o, reqId: data.reqId }, '*')
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
							: undefined,
						force_viewer_sensitive_inputs: editor
							? undefinedIfEmpty(
									Object.keys(runnable?.fields ?? {}).filter(
										(k) => runnable?.fields?.[k]?.type == 'user' && runnable?.fields?.[k]?.sensitive
									)
								)
							: undefined,
						force_viewer_delete_after_secs: editor ? runnable?.delete_after_secs : undefined
					},
					undefined
				)
				launchedJobs.add(uuid)
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
			if (gateJobIds && !launchedJobs.has(data.jobId)) {
				respond({ result: { message: 'Unknown job' }, error: true })
				return
			}
			await respondWithResult(data.jobId)
		} else if (event.data.type == 'getJob') {
			if (gateJobIds && !launchedJobs.has(data.jobId)) {
				respond({ result: { message: 'Unknown job' }, error: true })
				return
			}
			const job = await JobService.getJob({ workspace, id: data.jobId })
			respond({ result: job })
		} else if (event.data.type == 'streamJob') {
			// Stream job results using SSE
			const jobId = data.jobId
			const reqId = data.reqId
			if (gateJobIds && !launchedJobs.has(jobId)) {
				sourceWindow?.postMessage(
					{ type: 'streamJobRes', reqId, error: true, result: { message: 'Unknown job' } },
					'*'
				)
				return
			}
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
						sourceWindow?.postMessage(
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
						sourceWindow?.postMessage(
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
						sourceWindow?.postMessage(
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
						sourceWindow?.postMessage(
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
				sourceWindow?.postMessage(
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
