<script lang="ts">
	import { type Job, JobService, type FlowStatus, type Preview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, tick } from 'svelte'
	import { createEventDispatcher } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import { isScriptPreview } from '$lib/utils'

	export let isLoading = false
	export let job: Job | undefined = undefined
	export let workspaceOverride: string | undefined = undefined
	export let notfound = false
	export let jobUpdateLastFetch: Date | undefined = undefined
	export let toastError = false
	export let lazyLogs = false
	// Will be set to number if job is not a flow
	// If you want to find out progress of subjobs of a flow, check job.flow_status.progress
	export let scriptProgress: number | undefined = undefined

	/// Last time asked for job progress
	let lastTimeCheckedProgress: number | undefined = undefined

	/// Will try to poll progress every 5s and if once progress returned was not undefined, will be ignored
	/// and getProgressRate will be used instead
	const getProgressRetryRate: number = 5000
	/// How often loader poll progress
	const getProgressRate: number = 1000

	const dispatch = createEventDispatcher()

	$: workspace = workspaceOverride ?? $workspaceStore

	let syncIteration: number = 0
	let errorIteration = 0

	let logOffset = 0

	let ITERATIONS_BEFORE_SLOW_REFRESH = 10
	let ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

	let lastStartedAt: number = Date.now()
	let currentId: string | undefined = undefined

	$: isLoading = currentId !== undefined

	export async function abstractRun(fn: () => Promise<string>) {
		try {
			isLoading = true
			clearCurrentJob()
			const startedAt = Date.now()
			const testId = await fn()

			if (lastStartedAt < startedAt) {
				lastStartedAt = startedAt
				if (testId) {
					try {
						await watchJob(testId)
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
			// if error happens on submitting the job, reset UI state so the user can try again
			isLoading = false
			currentId = undefined
			job = undefined
			throw err
		}
	}

	export async function runScriptByPath(
		path: string | undefined,
		args: Record<string, any>
	): Promise<string> {
		return abstractRun(() =>
			JobService.runScriptByPath({
				workspace: $workspaceStore!,
				path: path ?? '',
				requestBody: args,
				skipPreprocessor: true
			})
		)
	}

	export async function runFlowByPath(
		path: string | undefined,
		args: Record<string, any>
	): Promise<string> {
		return abstractRun(() =>
			JobService.runFlowByPath({
				workspace: $workspaceStore!,
				path: path ?? '',
				requestBody: args,
				skipPreprocessor: true
			})
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
		hash?: string
	): Promise<string> {
		// Reset in case we rerun job without reloading
		scriptProgress = undefined
		lastTimeCheckedProgress = undefined

		return abstractRun(() =>
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
			})
		)
	}

	export async function cancelJob() {
		const id = currentId
		if (id) {
			dispatch('cancel', id)
			currentId = undefined
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
		if (currentId) {
			job = undefined
			await cancelJob()
		}
	}

	export async function watchJob(testId: string) {
		logOffset = 0
		syncIteration = 0
		errorIteration = 0
		currentId = testId
		job = undefined
		const isCompleted = await loadTestJob(testId)
		if (!isCompleted) {
			setTimeout(() => {
				syncer(testId)
			}, 50)
		}
	}

	async function loadTestJob(id: string): Promise<boolean> {
		let isCompleted = false
		if (currentId === id) {
			try {
				if (job && `running` in job) {
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

					const offset = logOffset == 0 ? (job.logs?.length ? job.logs?.length + 1 : 0) : logOffset

					let previewJobUpdates = await JobService.getJobUpdates({
						workspace: workspace!,
						id,
						running: job.running,
						logOffset: offset,
						getProgress: getProgress
					})

					// Clamp number between two values with the following line:
					const clamp = (num, min, max) => Math.min(Math.max(num, min), max)

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
					if ((previewJobUpdates.running ?? false) || (previewJobUpdates.completed ?? false)) {
						job = await JobService.getJob({ workspace: workspace!, id })
					}
				} else {
					job = await JobService.getJob({ workspace: workspace!, id, noLogs: lazyLogs })
				}
				jobUpdateLastFetch = new Date()

				if (job?.type === 'CompletedJob') {
					//only CompletedJob has success property
					isCompleted = true
					if (currentId === id) {
						await tick()
						dispatch('done', job)
						currentId = undefined
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
			}
			return isCompleted
		} else {
			return true
		}
	}

	async function syncer(id: string): Promise<void> {
		if (currentId != id) {
			dispatch('cancel', id)
			return
		}
		syncIteration++
		if (await loadTestJob(id)) {
			return
		}
		let nextIteration = 50
		if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
			nextIteration = 500
		} else if (syncIteration > ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
			nextIteration = 2000
		}
		setTimeout(() => syncer(id), nextIteration)
	}

	onDestroy(async () => {
		currentId = undefined
	})
</script>

<slot
	{job}
	{isLoading}
	{workspaceOverride}
	{notfound}
	{abstractRun}
	{runScriptByPath}
	{runFlowByPath}
	{runPreview}
	{cancelJob}
	{clearCurrentJob}
	{watchJob}
	{loadTestJob}
	{syncer}
/>
