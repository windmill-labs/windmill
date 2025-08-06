<script lang="ts">
	import {
		type Job,
		JobService,
		type FlowStatus,
		type Preview,
		type GetJobUpdatesResponse,
		type WorkflowStatus
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, tick, untrack } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import { isScriptPreview } from '$lib/utils'

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
		scriptProgress = $bindable(undefined),
		noLogs = false,
		children
	}: Props = $props()

	/// Last time asked for job progress
	let lastTimeCheckedProgress: number | undefined = undefined

	/// Will try to poll progress every 5s and if once progress returned was not undefined, will be ignored
	/// and getProgressRate will be used instead
	const getProgressRetryRate: number = 5000
	/// How often loader poll progress
	const getProgressRate: number = 1000

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
	let noPingTimeout: NodeJS.Timeout | undefined = undefined

	$effect(() => {
		let newIsLoading = currentId !== undefined
		untrack(() => {
			if (isLoading !== newIsLoading) {
				isLoading = newIsLoading
			}
		})
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

	function refreshLogOffset() {
		if (logOffset == 0) {
			logOffset = job?.logs?.length ? job.logs?.length + 1 : 0
		}
		if (resultStreamOffset == 0) {
			resultStreamOffset = job?.result_stream?.length ? job.result_stream?.length + 1 : 0
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

	export async function runPreview(
		path: string | undefined,
		code: string,
		lang: SupportedLanguage | undefined,
		args: Record<string, any>,
		tag: string | undefined,
		lock?: string,
		hash?: string,
		callbacks?: Callbacks
	): Promise<string> {
		// Reset in case we rerun job without reloading
		scriptProgress = undefined
		lastTimeCheckedProgress = undefined

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
						script_hash: hash
					}
				}),
			callbacks
		)
	}

	export async function cancelJob() {
		const id = currentId
		if (id) {
			lastCallbacks?.cancel?.({ id })
			lastCallbacks = undefined
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
			lastCallbacks = undefined
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

	let startedWatchingJob: number | undefined = undefined
	export async function watchJob(testId: string, callbacks?: Callbacks) {
		logOffset = 0
		resultStreamOffset = 0
		syncIteration = 0
		errorIteration = 0
		currentId = testId
		job = undefined
		startedWatchingJob = Date.now()

		// Clean up any existing SSE connection
		currentEventSource?.close()
		currentEventSource = undefined

		// Try SSE first, fall back to polling if needed
		if (supportsSSE()) {
			await loadTestJobWithSSE(testId, 0, callbacks)
		} else {
			syncer(testId, callbacks)
		}
	}

	function setJobProgress(job: Job) {
		let getProgress: boolean | undefined = undefined

		// We only pull individual job progress this way
		// Flow's progress we are getting from FlowStatusModule of flow job
		if (job.job_kind == 'script' || isScriptPreview(job.job_kind)) {
			// First time, before running job, lastTimeCheckedProgress is always undefined
			if (lastTimeCheckedProgress) {
				const lastTimeCheckedMs = Date.now() - lastTimeCheckedProgress
				// Ask for progress if the last time we asked is >5s OR the progress was once not undefined
				if (
					lastTimeCheckedMs > getProgressRetryRate ||
					(scriptProgress != undefined && lastTimeCheckedMs > getProgressRate)
				) {
					lastTimeCheckedProgress = Date.now()
					getProgress = true
				}
			} else {
				// Make it think we asked for progress, but in reality we didnt. First 5s we want to wait without putting extra work on db
				// 99.99% of the jobs won't have progress be set so we have to do a balance between having low-latency for jobs that use it and job that don't
				// we would usually not care to have progress the first 5s and jobs that are less than 5s
				lastTimeCheckedProgress = Date.now()
			}
		}
		return getProgress
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
				previewJobUpdates.new_logs ||
				previewJobUpdates.flow_status ||
				previewJobUpdates.mem_peak)
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
					let getProgress: boolean | undefined = setJobProgress(job)

					refreshLogOffset()

					let previewJobUpdates = await JobService.getJobUpdates({
						workspace: workspace!,
						id,
						running: job.running,
						logOffset: logOffset,
						getProgress: getProgress
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
				if (!job && !onlyResult) {
					job = await JobService.getJob({
						workspace: workspace!,
						id,
						noLogs: noLogs,
						noCode
					})
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
						onlyResult || !job ? undefined : setJobProgress(job)

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
								noPingTimeout = undefined
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
						if (attempt < 3) {
							console.log(`SSE error (1), retrying ...  attempt: ${attempt + 1}/3`)
							setTimeout(() => loadTestJobWithSSE(id, attempt + 1, callbacks), 1000)
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
