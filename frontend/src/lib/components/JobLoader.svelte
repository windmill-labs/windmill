<script lang="ts">
	import {
		type Job,
		JobService,
		type FlowStatus,
		type Preview,
		type GetJobUpdatesResponse
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, tick, untrack } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import { isScriptPreview } from '$lib/utils'

	// Will be set to number if job is not a flow

	type Callbacks = {
		done?: (x: Job & { result?: any }) => void
		doneError?: (x: { id: string; error: Error }) => void
		cancel?: () => void
		error?: (err: Error) => void
		started?: (id: string) => void
		running?: (id: string) => void
	}

	interface Props {
		isLoading?: boolean
		job?: Job | undefined
		noCode?: boolean
		workspaceOverride?: string | undefined
		notfound?: boolean
		allowConcurentRequests?: boolean
		jobUpdateLastFetch?: Date | undefined
		toastError?: boolean
		lazyLogs?: boolean
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
		lazyLogs = false,
		onlyResult = false,
		scriptProgress = $bindable(undefined),
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
	let lastCallbacks: Callbacks | undefined = undefined

	let finished: string[] = []
	let ITERATIONS_BEFORE_SLOW_REFRESH = 10
	let ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

	let lastStartedAt: number = Date.now()
	let currentId: string | undefined = $state(undefined)

	$effect(() => {
		let newIsLoading = currentId !== undefined
		untrack(() => {
			if (isLoading !== newIsLoading) {
				isLoading = newIsLoading
			}
		})
	})

	export async function abstractRun(fn: () => Promise<string>, callbacks?: Callbacks) {
		try {
			isLoading = true
			clearCurrentJob()
			lastCallbacks = callbacks
			const startedAt = Date.now()
			const testId = await fn()

			if (lastStartedAt < startedAt || allowConcurentRequests) {
				lastStartedAt = startedAt
				if (testId) {
					callbacks?.started?.(testId)
					try {
						await watchJob(testId, callbacks)
					} catch {
						if (currentId === testId) {
							currentId = undefined
						}
					}
				}
			}
			return testId
		} catch (err) {
			if (toastError) {
				sendUserToast(err.body, true)
			}
			callbacks?.error?.(err)
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

	export async function getLogs() {
		if (job) {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspace!,
				id: job.id,
				running: `running` in job && job.running,
				logOffset: job.logs?.length ?? 0
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
			lastCallbacks?.cancel?.()
			lastCallbacks = undefined
			currentId = undefined
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
			lastCallbacks?.cancel?.()
			lastCallbacks = undefined
			await cancelJob()
		}
	}

	export async function watchJob(testId: string, callbacks?: Callbacks) {
		logOffset = 0
		syncIteration = 0
		errorIteration = 0
		currentId = testId
		job = undefined

		// Clean up any existing SSE connection
		currentEventSource?.close()
		currentEventSource = undefined

		// Try SSE first, fall back to polling if needed
		await loadTestJobWithSSE(testId, 0, callbacks)
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
		offset: number,
		job: Job
	) {
		// Clamp number between two values with the following line:

		if (previewJobUpdates.progress) {
			// Progress cannot go back and cannot be set to 100
			scriptProgress = clamp(previewJobUpdates.progress, scriptProgress ?? 0, 99)
		}

		if (previewJobUpdates.new_logs) {
			if (offset == 0) {
				job.logs = previewJobUpdates.new_logs ?? ''
			} else {
				job.logs = (job?.logs ?? '').concat(previewJobUpdates.new_logs)
			}
		}

		if (previewJobUpdates.log_offset) {
			logOffset = previewJobUpdates.log_offset ?? 0
		}

		if (previewJobUpdates.flow_status) {
			job.flow_status = previewJobUpdates.flow_status as FlowStatus
		}
		if (previewJobUpdates.mem_peak && job) {
			job.mem_peak = previewJobUpdates.mem_peak
		}
	}
	async function loadTestJob(id: string, callbacks?: Callbacks): Promise<boolean> {
		let isCompleted = false
		if (currentId === id || allowConcurentRequests) {
			try {
				if (job && `running` in job) {
					callbacks?.running?.(id)
					let getProgress: boolean | undefined = setJobProgress(job)

					const offset = logOffset == 0 ? (job.logs?.length ? job.logs?.length + 1 : 0) : logOffset

					let previewJobUpdates = await JobService.getJobUpdates({
						workspace: workspace!,
						id,
						running: job.running,
						logOffset: offset,
						getProgress: getProgress
					})

					if ((previewJobUpdates.running ?? false) || (previewJobUpdates.completed ?? false)) {
						job = await JobService.getJob({ workspace: workspace!, id, noCode })
					}

					updateJobFromProgress(previewJobUpdates, offset, job)
				} else {
					job = await JobService.getJob({ workspace: workspace!, id, noLogs: lazyLogs, noCode })
				}
				jobUpdateLastFetch = new Date()

				if (job?.type === 'CompletedJob') {
					//only CompletedJob has success property
					isCompleted = true
					onJobCompleted(id, job, callbacks)
				}
				notfound = false
			} catch (err) {
				errorIteration += 1
				if (errorIteration == 5) {
					notfound = true
					job = undefined
				}
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
		if (currentId === id || allowConcurentRequests) {
			await tick()
			if (
				(callbacks?.error || callbacks?.doneError) &&
				!job?.success &&
				typeof job?.result == 'object' &&
				'error' in (job?.result ?? {})
			) {
				callbacks?.error?.(job.result.error)
				callbacks?.doneError?.({
					id,
					error: job.result.error
				})
			} else {
				callbacks?.done?.(job)
			}

			if (!allowConcurentRequests) {
				currentId = undefined
			} else {
				finished.push(id)
			}
		}
	}
	async function loadTestJobWithSSE(
		id: string,
		attempt: number,
		callbacks?: Callbacks
	): Promise<boolean> {
		let isCompleted = false
		if (currentId === id) {
			try {
				// First load the job to get initial state
				if (!job) {
					job = await JobService.getJob({ workspace: workspace!, id, noLogs: lazyLogs, noCode })
				}

				// If job is already completed, don't start SSE
				if (job?.type === 'CompletedJob') {
					isCompleted = true
					if (currentId === id) {
						await tick()
						callbacks?.done?.(job)
						currentId = undefined
					}
					return isCompleted
				} else if (job?.type === 'QueuedJob' && !currentEventSource) {
					if (job.running) {
						callbacks?.running?.(id)
					}

					let getProgress: boolean | undefined = setJobProgress(job)

					const offset = logOffset == 0 ? (job.logs?.length ? job.logs?.length + 1 : 0) : logOffset

					// Build SSE URL with query parameters
					const params = new URLSearchParams({
						running: job.running.toString(),
						log_offset: offset.toString()
					})
					if (getProgress !== undefined) {
						params.set('get_progress', getProgress.toString())
					}

					const sseUrl = `/api/w/${workspace}/jobs_u/getupdate_sse/${id}?${params.toString()}`

					currentEventSource = new EventSource(sseUrl)

					currentEventSource.onmessage = async (event) => {
						if (currentId !== id) {
							currentEventSource?.close()
							currentEventSource = undefined
							return
						}

						try {
							const previewJobUpdates = JSON.parse(event.data)
							jobUpdateLastFetch = new Date()
							if (previewJobUpdates.running) {
								callbacks?.running?.(id)
							}

							if (job) {
								updateJobFromProgress(previewJobUpdates, offset, job)
							}

							// Check if job is completed
							if (previewJobUpdates.completed) {
								const njob = previewJobUpdates.job as Job
								njob.logs = job?.logs ?? ''
								job = njob
								currentEventSource?.close()
								currentEventSource = undefined

								isCompleted = true
								onJobCompleted(id, job, callbacks)
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
							console.log(`SSE error )1), retrying ...  attempt: ${attempt}/3`)
							attempt++
							setTimeout(() => loadTestJobWithSSE(id, attempt, callbacks), 1000)
						} else {
							// Fall back to polling on error
							setTimeout(() => syncer(id), 1000)
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
					setTimeout(() => syncer(id), 1000)
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
		if ((currentId != id && !allowConcurentRequests) || finished.includes(id)) {
			callbacks?.cancel?.()
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
		currentId = undefined
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
