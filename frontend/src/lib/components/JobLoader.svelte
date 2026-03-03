<script module lang="ts">
	import pLimit from 'p-limit'

	const plimit = pLimit(5)
</script>

<script lang="ts">
	import {
		type Job,
		JobService,
		type FlowStatus,
		type Preview,
		type GetJobUpdatesResponse,
		type WorkflowStatus,
		type OpenFlow
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, onDestroy, tick, untrack } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import { DynamicInput, isScriptPreview } from '$lib/utils'
	import {
		getActiveRecording,
		getActiveReplay,
		getReplayStartTime
	} from './recording/flowRecording.svelte'

	// Will be set to number if job is not a flow

	export type Callbacks = {
		done?: (x: Job & { result?: any }) => void
		doneResult?: ({ id, result }: { id: string; result: any }) => void
		doneError?: ({ id, error }: { id?: string; error: Error }) => void
		change?: (x: Job) => void
		cancel?: ({ id }: { id: string }) => void
		started?: ({ id }: { id: string }) => void
		running?: ({ id }: { id: string }) => void
		resultStreamUpdate?: ({ id, result_stream }: { id: string; result_stream?: string }) => void
		loadExtraLogs?: ({ id, logs }: { id: string; logs: string }) => void
	}

	interface Props {
		isLoading?: boolean
		job?: (Job & { result_stream?: string }) | undefined
		noCode?: boolean
		noLogs?: boolean
		workspaceOverride?: string | undefined
		notfound?: boolean
		allowConcurentRequests?: boolean
		jobUpdateLastFetch?: Date | undefined
		toastError?: boolean
		onlyResult?: boolean
		loadPlaceholderJobOnStart?: Job
		// If you want to find out progress of subjobs of a flow, check job.flow_status.progress
		scriptProgress?: number | undefined

		children?: import('svelte').Snippet<[any]>
	}

	let {
		isLoading = $bindable(false),
		job = $bindable(undefined),
		noCode = false,
		allowConcurentRequests = false,
		workspaceOverride = undefined,
		notfound = $bindable(false),
		jobUpdateLastFetch = $bindable(undefined),
		toastError = false,
		onlyResult = false,
		loadPlaceholderJobOnStart = undefined,
		scriptProgress = $bindable(undefined),
		noLogs = false,
		children
	}: Props = $props()

	let workspace = $derived(workspaceOverride ?? $workspaceStore)

	let syncIteration: number = 0
	let errorIteration = 0

	let logOffset = 0
	let resultStreamOffset = 0
	let lastCallbacks: Callbacks | undefined = undefined

	let finished: string[] = []
	let ITERATIONS_BEFORE_SLOW_REFRESH = 10
	let ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

	let lastStartedAt: number = Date.now()
	let currentId: string | undefined = $state(undefined)
	let noPingTimeout: number | undefined = undefined
	let lastNoLogs = $state(noLogs)
	let lastCompletedJobId = $state<string | undefined>(undefined)

	let token = getContext<{ token?: string }>('AuthToken')

	$effect(() => {
		let newIsLoading = currentId !== undefined
		untrack(() => {
			if (isLoading !== newIsLoading) {
				isLoading = newIsLoading
			}
		})
	})

	const noLogsChangeRestartEvent = 'SSE restart after no logs change'
	$effect(() => {
		if (noLogs != lastNoLogs) {
			lastNoLogs = noLogs
			if (!noLogs) {
				currentEventSource?.onerror?.(new Event(noLogsChangeRestartEvent))
				const lastJobId = lastCompletedJobId
				if (lastJobId && (job || lastCallbacks?.loadExtraLogs)) {
					plimit(() =>
						JobService.getCompletedJobLogsTail({
							workspace: $workspaceStore!,
							id: lastJobId
						})
					).then((res) => {
						if (res && job) {
							job.logs = res
						}
						if (res && lastCallbacks?.loadExtraLogs) {
							lastCallbacks.loadExtraLogs({ id: lastJobId, logs: res })
						}
					})
				}
			}
		}
	})

	function clearCurrentId() {
		if (currentId) {
			if (allowConcurentRequests) {
				finished.push(currentId)
			} else {
				currentId = undefined
			}
		}
	}

	export async function abstractRun(fn: () => Promise<string>, callbacks?: Callbacks) {
		try {
			isLoading = true
			scriptProgress = undefined
			lastCompletedJobId = undefined
			clearCurrentJob()
			lastCallbacks = callbacks
			noPingTimeout = undefined
			const startedAt = Date.now()
			const testId = await fn()

			if (lastStartedAt < startedAt || allowConcurentRequests) {
				lastStartedAt = startedAt
				if (testId) {
					callbacks?.started?.({ id: testId })
					try {
						await watchJob(testId, callbacks)
					} catch {
						if (currentId === testId) {
							clearCurrentId()
						}
					}
				}
			}
			return testId
		} catch (err) {
			if (toastError) {
				sendUserToast(err.body, true)
			}
			callbacks?.doneError?.({ error: err })
			// if error happens on submitting the job, reset UI state so the user can try again
			isLoading = false
			currentId = undefined
			job = undefined
			throw err
		}
	}

	export async function runScriptByPath(
		path: string,
		args: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runScriptByPath({
					workspace: $workspaceStore!,
					path: path ?? '',
					requestBody: args,
					skipPreprocessor: true
				}),
			callbacks
		)
	}

	export async function runScriptByHash(
		hash: string,
		args: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runScriptByHash({
					workspace: $workspaceStore!,
					hash: hash ?? '',
					requestBody: args,
					skipPreprocessor: true
				}),
			callbacks
		)
	}

	export async function runFlowByPath(
		path: string,
		args: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runFlowByPath({
					workspace: $workspaceStore!,
					path: path ?? '',
					requestBody: args,
					skipPreprocessor: true
				}),
			callbacks
		)
	}

	export async function runFlowPreview(
		args: Record<string, any>,
		flow: OpenFlow & { tag?: string },
		callbacks?: Callbacks,
		path?: string
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runFlowPreview({
					workspace: $workspaceStore!,
					requestBody: {
						args,
						value: flow.value,
						tag: flow.tag,
						path
					}
				}),
			callbacks
		)
	}

	function refreshLogOffset() {
		if (logOffset == 0) {
			logOffset = job?.logs?.length ? job.logs?.length + 1 : 0
		}
	}
	export async function getLogs() {
		if (job) {
			refreshLogOffset()
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspace!,
				id: job.id,
				running: `running` in job && job.running,
				logOffset: logOffset
			})

			if ((job?.logs ?? '').length == 0) {
				job.logs = getUpdate.new_logs ?? ''
				logOffset = getUpdate.log_offset ?? 0
			}
		}
	}

	export async function runDynamicInputScript(
		entrypoint_function: string,
		runnable_ref: DynamicInput.HelperScript,
		args: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runDynamicSelect({
					workspace: $workspaceStore!,
					requestBody: { entrypoint_function, args, runnable_ref }
				}),
			callbacks
		)
	}

	export async function runPreview(
		path: string | undefined,
		code: string,
		lang: SupportedLanguage | undefined,
		args: Record<string, any>,
		tag: string | undefined,
		lock?: string,
		hash?: string,
		callbacks?: Callbacks,
		flowPath?: string
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runScriptPreview({
					workspace: $workspaceStore!,
					requestBody: {
						path,
						content: code,
						args,
						language: lang as Preview['language'],
						tag,
						lock,
						script_hash: hash,
						flow_path: flowPath
					}
				}),
			callbacks
		)
	}

	export async function cancelJob() {
		const id = currentId
		if (id) {
			lastCallbacks?.cancel?.({ id })
			clearCurrentId()
			// Clean up SSE connection
			currentEventSource?.close()
			currentEventSource = undefined
			try {
				await JobService.cancelQueuedJob({
					workspace: $workspaceStore ?? '',
					id,
					requestBody: {}
				})
			} catch (err) {
				console.error(err)
			}
		}
	}

	export async function clearCurrentJob() {
		if (currentId && !allowConcurentRequests) {
			job = undefined
			lastCallbacks?.cancel?.({ id: currentId })
			await cancelJob()
		}
	}

	function supportsSSE() {
		const sseSupported = typeof EventSource !== 'undefined'
		if (!sseSupported) {
			console.error('SSE not supported, falling back to polling')
		}
		return sseSupported
	}

	let replayTimeouts: ReturnType<typeof setTimeout>[] = []

	let startedWatchingJob: number | undefined = undefined
	export async function watchJob(testId: string, callbacks?: Callbacks) {
		logOffset = 0
		resultStreamOffset = 0
		syncIteration = 0
		errorIteration = 0
		currentId = testId
		scriptProgress = undefined

		// Replay mode: feed recorded events instead of real SSE
		const replay = getActiveReplay()
		if (replay) {
			const recorded = replay.jobs[testId]
			if (recorded) {
				job = structuredClone(recorded.initial_job)
				callbacks?.change?.(job)
				// Compute delays relative to replay start so sub-jobs (discovered
				// later by FlowStatusViewerInner) stay in sync with the root job.
				const elapsed = Date.now() - getReplayStartTime()
				for (const event of recorded.events) {
					const delay = Math.max(0, event.t - elapsed)
					const timeout = setTimeout(() => {
						if (job) {
							updateJobFromProgress(event.data, job, callbacks)
						}
						if (event.data.completed) {
							const njob = (event.data as any).job as Job & { result_stream?: string }
							if (njob) {
								// Use whichever logs are more complete (longer)
								const streamedLogs = job?.logs ?? ''
								const completedLogs = njob.logs ?? ''
								njob.logs =
									streamedLogs.length >= completedLogs.length ? streamedLogs : completedLogs
								const streamedResult = job?.result_stream ?? ''
								const completedResult = njob.result_stream ?? ''
								njob.result_stream =
									streamedResult.length >= completedResult.length ? streamedResult : completedResult
								job = njob
								onJobCompleted(testId, job, callbacks)
							}
						}
					}, delay)
					replayTimeouts.push(timeout)
				}
				return
			}
			// Job not in recording — stub it to prevent API calls
			const stubJob = { id: testId, type: 'CompletedJob', success: true } as Job
			job = stubJob
			callbacks?.change?.(stubJob)
			clearCurrentId()
			return
		}

		if (loadPlaceholderJobOnStart) {
			job = structuredClone(loadPlaceholderJobOnStart)
		} else {
			job = undefined
		}
		startedWatchingJob = Date.now()

		// Clean up any existing SSE connection
		currentEventSource?.close()
		currentEventSource = undefined
		lastCallbacks = callbacks
		// Try SSE first, fall back to polling if needed
		if (supportsSSE()) {
			await loadTestJobWithSSE(testId, 0, callbacks)
		} else {
			syncer(testId, callbacks)
		}
	}

	const clamp = (num: number, min: number, max: number) => Math.min(Math.max(num, min), max)

	function updateJobFromProgress(
		previewJobUpdates: GetJobUpdatesResponse,
		job: Job & { result_stream?: string },
		callbacks: Callbacks | undefined
	) {
		// Clamp number between two values with the following line:
		if (previewJobUpdates.running) {
			if (job && job.type == 'QueuedJob' && !job.running) {
				callbacks?.running?.({ id: job.id })
				job.running = true
			}
		}
		if (previewJobUpdates.progress) {
			console.log('progress', previewJobUpdates.progress)
			// Progress cannot go back and cannot be set to 100
			scriptProgress = clamp(previewJobUpdates.progress, scriptProgress ?? 0, 99)
		}

		if (previewJobUpdates.new_logs) {
			if (logOffset == 0) {
				job.logs = previewJobUpdates.new_logs ?? ''
			} else {
				job.logs = (job?.logs ?? '').concat(previewJobUpdates.new_logs)
			}
		}

		if (previewJobUpdates.new_result_stream) {
			if (!job.result_stream) {
				job.result_stream = previewJobUpdates.new_result_stream
			} else {
				job.result_stream = job.result_stream.concat(previewJobUpdates.new_result_stream)
			}
			callbacks?.resultStreamUpdate?.({
				id: job.id,
				result_stream: job.result_stream
			})
		}

		if (previewJobUpdates.log_offset) {
			logOffset = previewJobUpdates.log_offset ?? 0
		}

		if (previewJobUpdates.stream_offset) {
			resultStreamOffset = previewJobUpdates.stream_offset ?? 0
		}

		if (previewJobUpdates.flow_status) {
			job.flow_status = previewJobUpdates.flow_status as FlowStatus
		}
		if (previewJobUpdates.workflow_as_code_status) {
			job.workflow_as_code_status = previewJobUpdates.workflow_as_code_status as WorkflowStatus
		}
		if (previewJobUpdates.mem_peak && job) {
			job.mem_peak = previewJobUpdates.mem_peak
		}

		if (
			job &&
			(previewJobUpdates.running ||
				previewJobUpdates.progress ||
				previewJobUpdates.log_offset ||
				previewJobUpdates.flow_status ||
				previewJobUpdates.mem_peak ||
				previewJobUpdates.stream_offset)
		) {
			callbacks?.change?.(job)
		}
	}
	async function loadTestJob(id: string, callbacks?: Callbacks): Promise<boolean> {
		let isCompleted = false
		if (isCurrentJob(id)) {
			try {
				if (job && `running` in job) {
					callbacks?.running?.({ id })

					refreshLogOffset()

					let previewJobUpdates = await JobService.getJobUpdates({
						workspace: workspace!,
						id,
						running: job.running,
						logOffset: logOffset,
						streamOffset: resultStreamOffset,
						getProgress: false
					})

					if ((previewJobUpdates.running ?? false) || (previewJobUpdates.completed ?? false)) {
						job = await JobService.getJob({
							workspace: workspace!,
							id,
							noCode,
							noLogs: onlyResult || noLogs
						})
						callbacks?.change?.(job)
					}

					updateJobFromProgress(previewJobUpdates, job, callbacks)
				} else {
					job = await JobService.getJob({
						workspace: workspace!,
						id,
						noLogs: onlyResult || noLogs,
						noCode
					})
				}
				jobUpdateLastFetch = new Date()

				if (job?.type === 'CompletedJob') {
					//only CompletedJob has success property
					isCompleted = true
					if (onlyResult) {
						callbacks?.doneResult?.({
							id,
							result: job?.result
						})
						clearCurrentId()
					} else {
						onJobCompleted(id, job, callbacks)
					}
				}
				notfound = false
			} catch (err) {
				errorIteration += 1
				if (errorIteration == 5) {
					notfound = true
					job = undefined
					clearCurrentId()
				}
				callbacks?.doneError?.({ error: err, id })
				console.warn(err)
			}
			return isCompleted
		} else {
			return true
		}
	}

	let currentEventSource: EventSource | undefined = undefined

	async function onJobCompleted(
		id: string,
		job: Job & { result?: any; success?: boolean },
		callbacks?: Callbacks
	) {
		if (isCurrentJob(id)) {
			await tick()
			if (
				callbacks?.doneError &&
				!job?.success &&
				typeof job?.result == 'object' &&
				'error' in (job?.result ?? {})
			) {
				callbacks?.doneError?.({
					id,
					error: job.result.error
				})
			} else {
				callbacks?.done?.(job)
			}
			callbacks?.change?.(job)

			clearCurrentId()
			lastCompletedJobId = id
		}
	}

	function setNoPingTimeout(id: string, attempt: number, callbacks?: Callbacks) {
		if (noPingTimeout) {
			clearTimeout(noPingTimeout)
		}
		if (isCurrentJob(id)) {
			noPingTimeout = setTimeout(() => {
				if (isCurrentJob(id)) {
					currentEventSource?.close()
					currentEventSource = undefined
					loadTestJobWithSSE(id, attempt + 1, callbacks)
				}
			}, 10000)
		}
	}

	function isCurrentJob(id: string) {
		return currentId === id || (allowConcurentRequests && !finished.includes(id))
	}

	async function loadTestJobWithSSE(
		id: string,
		attempt: number,
		callbacks?: Callbacks
	): Promise<boolean> {
		let isCompleted = false
		let resultOnlyResultStream: string = ''
		if (isCurrentJob(id)) {
			try {
				// First load the job to get initial state
				if ((!job || job.id == '') && !onlyResult) {
					job = await JobService.getJob({
						workspace: workspace!,
						id,
						noLogs: noLogs,
						noCode
					})

					callbacks?.change?.(job)
					getActiveRecording()?.recordInitialJob(id, job)
				}

				if (!onlyResult) {
					callbacks?.resultStreamUpdate?.({
						id,
						result_stream: undefined
					})
				}

				// If job is already completed, don't start SSE
				if (job?.type === 'CompletedJob') {
					isCompleted = true
					if (isCurrentJob(id)) {
						onJobCompleted(id, job, callbacks)
					}
					return isCompleted
				} else if (onlyResult || (job?.type === 'QueuedJob' && !currentEventSource)) {
					if (job?.type === 'QueuedJob' && job?.running) {
						callbacks?.running?.({ id })
						if (!job.running) {
							job.running = true
						}
					}

					let getProgress: boolean | undefined =
						onlyResult || !job
							? undefined
							: job.job_kind == 'script' || isScriptPreview(job.job_kind)

					refreshLogOffset()
					// Build SSE URL with query parameters
					const params = new URLSearchParams({
						log_offset: logOffset.toString()
					})
					if (job && job.type === 'QueuedJob') {
						params.set('running', job.running.toString())
					} else if (onlyResult && callbacks?.running) {
						params.set('running', 'false')
					}

					if (getProgress !== undefined) {
						params.set('get_progress', getProgress.toString())
					}
					if (onlyResult) {
						params.set('only_result', 'true')
					}

					if (startedWatchingJob && startedWatchingJob > Date.now() - 5000) {
						params.set('fast', 'true')
					}
					if (noLogs) {
						params.set('no_logs', 'true')
					}
					if (resultStreamOffset) {
						params.set('stream_offset', resultStreamOffset.toString())
					}
					if (job) {
						params.set(
							'is_flow',
							(job.job_kind === 'flow' || job.job_kind === 'flowpreview').toString()
						)
					}

					if (token?.token && token.token != '') {
						params.set('token', token.token)
					}

					const sseUrl = `/api/w/${workspace}/jobs_u/getupdate_sse/${id}?${params.toString()}`

					currentEventSource = new EventSource(sseUrl)

					setNoPingTimeout(id, attempt, callbacks)
					currentEventSource.onmessage = async (event) => {
						if (!isCurrentJob(id)) {
							currentEventSource?.close()
							currentEventSource = undefined
							return
						}

						try {
							const previewJobUpdates = JSON.parse(event.data)
							let type = previewJobUpdates.type
							if (type == 'timeout') {
								currentEventSource?.close()
								currentEventSource = undefined
								loadTestJobWithSSE(id, 0, callbacks)
								return
							} else if (type == 'ping') {
								setNoPingTimeout(id, attempt, callbacks)
								return
							} else if (type == 'error') {
								currentEventSource?.close()
								currentEventSource = undefined
								console.error('SSE error:', previewJobUpdates)
								throw new Error('SSE error: ' + previewJobUpdates)
							} else if (type == 'not_found') {
								currentEventSource?.close()
								currentEventSource = undefined
								console.error('Not found')
								throw new Error('Not found')
							}
							jobUpdateLastFetch = new Date()
							getActiveRecording()?.recordEvent(id, previewJobUpdates)

							if (job) {
								updateJobFromProgress(previewJobUpdates, job, callbacks)
							} else if (onlyResult && callbacks?.running && previewJobUpdates.running) {
								callbacks?.running?.({ id })
							}

							if (onlyResult && previewJobUpdates.new_result_stream) {
								resultOnlyResultStream = resultOnlyResultStream.concat(
									previewJobUpdates.new_result_stream
								)
								// console.log('resultOnlyResultStream', resultOnlyResultStream)
								callbacks?.resultStreamUpdate?.({
									id,
									result_stream: resultOnlyResultStream
								})
							}

							// Check if job is completed
							if (previewJobUpdates.completed) {
								currentEventSource?.close()
								currentEventSource = undefined
								if (noPingTimeout) {
									clearTimeout(noPingTimeout)
									noPingTimeout = undefined
								}
								isCompleted = true
								if (onlyResult) {
									callbacks?.doneResult?.({
										id,
										result: previewJobUpdates?.only_result
									})
									clearCurrentId()
								} else {
									const njob = previewJobUpdates.job as Job & { result_stream?: string }
									njob.logs = job?.logs ?? ''
									njob.result_stream = job?.result_stream ?? ''
									job = njob
									onJobCompleted(id, job, callbacks)
								}
							}
						} catch (parseErr) {
							console.warn('Failed to parse SSE data:', parseErr)
						}
					}

					currentEventSource.onerror = (error) => {
						console.warn('SSE error:', error)
						currentEventSource?.close()
						currentEventSource = undefined
						let delay = 1000
						let isNoLogsChange = error.type == noLogsChangeRestartEvent
						if (isNoLogsChange) {
							delay = 0
						}
						if (attempt < 3 || isNoLogsChange) {
							if (!isNoLogsChange) {
								console.log(`SSE error (1), retrying ...  attempt: ${attempt + 1}/3`)
							}
							setTimeout(() => loadTestJobWithSSE(id, attempt + 1, callbacks), delay)
						} else {
							// Fall back to polling on error
							setTimeout(() => syncer(id, callbacks), 1000)
						}
					}

					currentEventSource.onopen = () => {
						console.log('SSE connection opened for job:', id)
					}
				}

				notfound = false
			} catch (err) {
				errorIteration += 1
				if (errorIteration == 5) {
					notfound = true
					job = undefined
				}
				console.warn(err)
				// Fall back to polling on error
				currentEventSource?.close()
				currentEventSource = undefined

				if (attempt < 3) {
					console.log(`SSE error (2), retrying ... attempt: ${attempt}/3`)
					attempt++
					loadTestJobWithSSE(id, attempt, callbacks)
				} else {
					// Fall back to polling on error
					setTimeout(() => syncer(id, callbacks), 1000)
				}
			}
			return isCompleted
		} else {
			// Clean up SSE connection if current ID changed
			currentEventSource?.close()
			currentEventSource = undefined
			return true
		}
	}

	async function syncer(id: string, callbacks?: Callbacks): Promise<void> {
		if (!isCurrentJob(id)) {
			callbacks?.cancel?.({ id })
			return
		}
		syncIteration++
		if (await loadTestJob(id, callbacks)) {
			return
		}
		let nextIteration = 50
		if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
			nextIteration = 500
		} else if (syncIteration > ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
			nextIteration = 2000
		}
		setTimeout(() => syncer(id, callbacks), nextIteration)
	}

	onDestroy(async () => {
		clearCurrentId()
		currentEventSource?.close()
		currentEventSource = undefined
		replayTimeouts.forEach(clearTimeout)
		replayTimeouts = []
	})
</script>

{@render children?.({
	job,
	isLoading,
	workspaceOverride,
	notfound,
	abstractRun,
	runScriptByPath,
	runFlowByPath,
	runPreview,
	cancelJob,
	clearCurrentJob,
	watchJob,
	loadTestJob,
	syncer
})}
