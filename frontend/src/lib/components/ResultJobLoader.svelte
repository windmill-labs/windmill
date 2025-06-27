<script lang="ts">
	import { JobService, type Preview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, tick, untrack } from 'svelte'
	import { createEventDispatcher } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'

	interface Props {
		isLoading?: boolean
		job?: { completed: boolean; result: any; id: string; success?: boolean } | undefined
		workspaceOverride?: string | undefined
		notfound?: boolean
		isEditor?: boolean
		allowConcurentRequests?: boolean
	}

	let {
		isLoading = $bindable(false),
		job = $bindable(undefined),
		workspaceOverride = undefined,
		notfound = $bindable(false),
		isEditor = false,
		allowConcurentRequests = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let workspace = $derived(workspaceOverride ?? $workspaceStore!)

	let syncIteration: number = 0
	let errorIteration = 0

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

	type Callbacks = { done: (x: any) => void; cancel: () => void; error: (err: Error) => void }

	let running = false
	let lastCallbacks: Callbacks | undefined = undefined

	let finished: string[] = []
	export async function abstractRun(fn: () => Promise<string>, callbacks?: Callbacks) {
		try {
			running = false
			isLoading = true
			clearCurrentJob()
			const startedAt = Date.now()
			const testId = await fn()
			lastCallbacks = callbacks
			if (lastStartedAt < startedAt || allowConcurentRequests) {
				lastStartedAt = startedAt
				if (testId) {
					dispatch('started', testId)
					try {
						await watchJob(testId, callbacks)
					} catch (e) {
						callbacks?.cancel()
						dispatch('cancel', testId)
						if (currentId === testId) {
							currentId = undefined
						}
					}
				}
			}
			return testId
		} catch (err) {
			callbacks?.error(err)
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
					workspace: workspace,
					path: path,
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
					workspace: workspace,
					hash: hash,
					requestBody: args,
					skipPreprocessor: true
				}),
			callbacks
		)
	}

	export async function runFlowByPath(
		path: string | undefined,
		args: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runFlowByPath({
					workspace: workspace,
					path: path ?? '',
					requestBody: args,
					skipPreprocessor: true
				}),
			callbacks
		)
	}

	export async function runPreview(
		path: string | undefined,
		code: string,
		lang: SupportedLanguage,
		args: Record<string, any>,
		tag: string | undefined,
		callbacks?: Callbacks
	): Promise<string> {
		return abstractRun(
			() =>
				JobService.runScriptPreview({
					workspace: workspace,
					requestBody: {
						path,
						content: code,
						args,
						language: lang as Preview['language'],
						tag
					}
				}),
			callbacks
		)
	}

	export async function cancelJob() {
		const id = currentId
		if (id) {
			lastCallbacks?.cancel()
			lastCallbacks = undefined

			dispatch('cancel', id)

			currentId = undefined
			try {
				await JobService.cancelQueuedJob({
					workspace: workspace ?? '',
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
			lastCallbacks?.cancel()
			dispatch('cancel', currentId)
			lastCallbacks = undefined
			job = undefined
			await cancelJob()
		}
	}

	export async function watchJob(testId: string, callbacks?: Callbacks) {
		syncIteration = 0
		errorIteration = 0
		currentId = testId
		job = undefined
		const isCompleted = await loadTestJob(testId, callbacks)
		if (!isCompleted) {
			setTimeout(() => {
				syncer(testId, callbacks)
			}, 50)
		}
	}

	async function loadTestJob(id: string, callbacks?: Callbacks): Promise<boolean> {
		let isCompleted = false
		if (currentId === id || allowConcurentRequests) {
			try {
				let maybe_job = await JobService.getCompletedJobResultMaybe({
					workspace: workspace ?? '',
					id,
					getStarted: isEditor
				})
				if (maybe_job.started && !running) {
					running = true
					dispatch('running', id)
				}
				if (maybe_job.completed) {
					isCompleted = true
					if (currentId === id || allowConcurentRequests) {
						job = { ...maybe_job, id }
						await tick()
						if (!job?.success && typeof job?.result == 'object' && 'error' in (job?.result ?? {})) {
							callbacks?.error(job.result.error)
							dispatch('doneError', {
								id,
								error: job.result.error
							})
						} else {
							callbacks?.done(job.result)
							dispatch('done', job)
						}
						finished.push(id)
						if (!allowConcurentRequests) {
							currentId = undefined
						}
					} else {
						callbacks?.cancel()
						dispatch('cancel', id)
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
			callbacks?.cancel()
			dispatch('cancel', id)
			return true
		}
	}

	async function syncer(id: string, callbacks?: Callbacks): Promise<void> {
		if ((currentId != id && !allowConcurentRequests) || finished.includes(id)) {
			callbacks?.cancel()
			dispatch('cancel', id)
			return
		}
		syncIteration++
		let r = await loadTestJob(id, callbacks)
		if (r) {
			return
		}
		let nextIteration = 50
		if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
			nextIteration = 500
		} else if (syncIteration > ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
			nextIteration = 2000
		}
		setTimeout(() => {
			syncer(id, callbacks)
		}, nextIteration)
	}

	onDestroy(async () => {
		currentId = undefined
	})
</script>
