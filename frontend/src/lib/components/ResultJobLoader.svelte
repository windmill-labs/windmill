<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, tick } from 'svelte'
	import type { Preview } from '$lib/gen/models/Preview'
	import { createEventDispatcher } from 'svelte'

	export let isLoading = false
	export let job: { completed: boolean; result: any; id: string } | undefined = undefined
	export let workspaceOverride: string | undefined = undefined
	export let notfound = false

	const dispatch = createEventDispatcher()

	$: workspace = workspaceOverride ?? $workspaceStore

	let syncIteration: number = 0
	let errorIteration = 0

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
					dispatch('started', testId)
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
		lang: 'deno' | 'go' | 'python3' | 'bash' | 'nativets',
		args: Record<string, any>,
		tag: string | undefined
	): Promise<string> {
		return abstractRun(() =>
			JobService.runScriptPreview({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					content: code,
					args,
					language: lang as Preview.language,
					tag
				}
			})
		)
	}

	export async function cancelJob() {
		const id = currentId
		if (id) {
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
				let maybe_job = await JobService.getCompletedJobResultMaybe({
					workspace: workspace ?? '',
					id
				})
				if (maybe_job.completed) {
					isCompleted = true
					if (currentId === id) {
						job = { ...maybe_job, id }
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
					await clearCurrentJob()
					dispatch('doneError', err)
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
			return
		}
		syncIteration++
		await loadTestJob(id)
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
