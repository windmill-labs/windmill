<script lang="ts">
	import { CompletedJob, Job, JobService } from '$lib/gen'
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
	let ITERATIONS_BEFORE_SLOW_REFRESH = 10
	let ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

	export async function abstractRun(fn: () => Promise<string>) {
		try {
			intervalId && (await clearIntervalAsync(intervalId))

			if (isLoading && job) {
				await JobService.cancelQueuedJob({
					workspace: workspace!,
					id: job.id,
					requestBody: {}
				})
			}
			isLoading = true

			const testId = await fn()

			await watchJob(testId)
		} catch (err) {
			isLoading = false
			throw err
		}
	}

	export async function runScriptByPath(
		path: string | undefined,
		args: Record<string, any>
	): Promise<void> {
		abstractRun(() =>
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
	): Promise<void> {
		abstractRun(() =>
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
	): Promise<void> {
		abstractRun(() =>
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

	export async function watchJob(testId: string) {
		intervalId && (await clearIntervalAsync(intervalId))
		job = undefined
		syncIteration = 0
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
					logOffset: job.logs?.length ?? 0
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
				intervalId && (await clearIntervalAsync(intervalId))
				if (isLoading) {
					dispatch('done', job)
					isLoading = false
				}
			}
			notfound = false
		} catch (err) {
			intervalId && (await clearIntervalAsync(intervalId))
			isLoading = false
			if (err.status === 404) {
				notfound = true
			}
			console.warn(err)
		}
		return isCompleted
	}

	async function syncer(id: string): Promise<void> {
		if (syncIteration == ITERATIONS_BEFORE_SLOW_REFRESH) {
			intervalId && (await clearIntervalAsync(intervalId))
			intervalId = setIntervalAsync(async () => await syncer(id), 500)
		} else if (syncIteration == ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
			intervalId && (await clearIntervalAsync(intervalId))
			intervalId = setIntervalAsync(async () => await syncer(id), 2000)
		}
		syncIteration++
		await loadTestJob(id)
	}

	onDestroy(async () => {
		intervalId && (await clearIntervalAsync(intervalId))
	})
</script>
