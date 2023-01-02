<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy } from 'svelte'
	import type { Preview } from '$lib/gen/models/Preview'
	import { createEventDispatcher } from 'svelte'
	import {
		setIntervalAsync,
		clearIntervalAsync,
		type SetIntervalAsyncTimer
	} from 'set-interval-async'

	export let isLoading = false
	export let job: Job | undefined = undefined
	export let workspaceOverride: string | undefined = undefined
	export let notfound = false

	const dispatch = createEventDispatcher()

	$: workspace = workspaceOverride ?? $workspaceStore
	let intervalId: SetIntervalAsyncTimer<unknown[]> | undefined = undefined

	let syncIteration: number = 0
	let errorIteration = 0

	let ITERATIONS_BEFORE_SLOW_REFRESH = 10
	let ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

	let stopCurrentIteration = false

	export async function abstractRun(fn: () => Promise<string>) {
		try {
			await clearCurrentJob()
			isLoading = true
			const testId = await fn()
			if (testId) {
				await watchJob(testId)
			}
			return testId
		} catch (err) {
			isLoading = false
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
				requestBody: args
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
				requestBody: args
			})
		)
	}

	export async function runPreview(
		path: string | undefined,
		code: string,
		lang: 'deno' | 'go' | 'python3' | 'bash',
		args: Record<string, any>
	): Promise<string> {
		return abstractRun(() =>
			JobService.runScriptPreview({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					content: code,
					args,
					language: lang as Preview.language
				}
			})
		)
	}

	export async function cancelJob() {
		try {
			await JobService.cancelQueuedJob({
				workspace: $workspaceStore ?? '',
				id: job?.id ?? '',
				requestBody: {}
			})
		} catch (err) {
			console.error(err)
		}
		isLoading = false
		console.log('cancelled')
	}

	export async function clearCurrentJob() {
		if (intervalId) {
			const interval = intervalId
			intervalId = undefined
			stopCurrentIteration = true
			if (isLoading && job) {
				await JobService.cancelQueuedJob({
					workspace: workspace!,
					id: job.id,
					requestBody: {}
				})
			}
			await clearIntervalAsync(interval)
		}
		stopCurrentIteration = false
		job = undefined
		isLoading = false
	}

	export async function watchJob(testId: string) {
		syncIteration = 0
		errorIteration = 0
		const isCompleted = await loadTestJob(testId)
		if (!isCompleted) {
			isLoading = true
			intervalId = setIntervalAsync(async () => {
				await syncer(testId)
			}, 50)
		}
	}

	async function loadTestJob(id: string): Promise<boolean> {
		let isCompleted = false
		try {
			if (job && `running` in job) {
				let previewJobUpdates = await JobService.getJobUpdates({
					workspace: workspace!,
					id,
					running: job.running,
					logOffset: job.logs?.length ? job.logs?.length + 1 : 0
				})

				if (previewJobUpdates.new_logs) {
					job.logs = (job?.logs ?? '').concat(previewJobUpdates.new_logs)
				}
				if ((previewJobUpdates.running ?? false) || (previewJobUpdates.completed ?? false)) {
					job = await JobService.getJob({ workspace: workspace!, id })
				}
			} else {
				job = await JobService.getJob({ workspace: workspace!, id })
			}
			job = await JobService.getJob({ workspace: workspace ?? '', id })

			if (job?.type === 'CompletedJob') {
				//only CompletedJob has success property
				isCompleted = true
				intervalId && clearIntervalAsync(intervalId!)
				if (isLoading) {
					dispatch('done', job)
					isLoading = false
				}
			}
			notfound = false
		} catch (err) {
			errorIteration += 1
			if (errorIteration == 5) {
				notfound = true
				await clearCurrentJob()
			}
			console.warn(err)
		}
		return isCompleted
	}

	async function syncer(id: string): Promise<void> {
		syncIteration++
		if (syncIteration == ITERATIONS_BEFORE_SLOW_REFRESH) {
			intervalId && clearIntervalAsync(intervalId!)
			intervalId = setIntervalAsync(async () => await syncer(id), 500)
		} else if (syncIteration == ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
			intervalId && clearIntervalAsync(intervalId!)
			intervalId = setIntervalAsync(async () => await syncer(id), 2000)
		}
		if (stopCurrentIteration) {
			return
		}
		await loadTestJob(id)
	}

	onDestroy(async () => {
		await clearCurrentJob()
	})
</script>
