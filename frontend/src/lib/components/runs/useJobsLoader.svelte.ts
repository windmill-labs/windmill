import { onDestroy, onMount, untrack } from 'svelte'
import {
	JobService,
	type Job,
	type CompletedJob,
	type ExtendedJobs,
	ConcurrencyGroupsService,
	type ObscuredJob,
	CancelablePromise,
	CancelError
} from '$lib/gen'

import { sendUserToast } from '$lib/toast'

import { tweened, type Tweened } from 'svelte/motion'
import { subtractDaysFromDateString } from '$lib/utils'
import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'
import type { Timeframe } from './TimeframeSelect.svelte'
import { allowWildcards as _allowWildcards, type RunsFilterInstance } from './runsFilter'

export function computeJobKinds(jobKindsCat: string | null): string {
	if (jobKindsCat == 'all') {
		return ''
	} else if (jobKindsCat == 'dependencies') {
		let kinds: CompletedJob['job_kind'][] = ['dependencies', 'flowdependencies', 'appdependencies']
		return kinds.join(',')
	} else if (jobKindsCat == 'previews') {
		let kinds: CompletedJob['job_kind'][] = ['preview', 'flowpreview']
		return kinds.join(',')
	} else if (jobKindsCat == 'deploymentcallbacks') {
		let kinds: CompletedJob['job_kind'][] = ['deploymentcallback']
		return kinds.join(',')
	} else if (jobKindsCat == 'runs') {
		let kinds: CompletedJob['job_kind'][] = ['script', 'flow', 'singlestepflow']
		return kinds.join(',')
	} else {
		let kinds: CompletedJob['job_kind'][] = [
			'script',
			'flow',
			'flowscript',
			'flownode',
			'appscript'
		]
		return kinds.join(',')
	}
}

export interface UseJobLoaderArgs {
	currentWorkspace: string
	filters?: Partial<RunsFilterInstance>
	timeframe?: Timeframe
	jobKinds?: string
	autoRefresh?: boolean
	argError?: string
	resultError?: string
	refreshRate?: number
	syncQueuedRunsCount?: boolean
	skip?: boolean
	lookback?: number
	perPage?: number
	onSetPerPage?: (perPage: number) => void
}

export function useJobsLoader(args: () => UseJobLoaderArgs) {
	let _args = $derived(args())

	let currentWorkspace = $derived(_args.currentWorkspace)
	let filters = $derived(_args.filters)
	let jobKinds = $derived(_args.jobKinds)
	let autoRefresh = $derived(_args.autoRefresh ?? true)
	let argError = $derived(_args.argError ?? '')
	let resultError = $derived(_args.resultError ?? '')
	let refreshRate = $derived(_args.refreshRate ?? 5000)
	let syncQueuedRunsCount = $derived(_args.syncQueuedRunsCount ?? true)
	let lookback = $derived(_args.lookback ?? 0)
	let onSetPerPage = $derived(_args.onSetPerPage)
	let timeframe = $derived(_args?.timeframe)
	let perPage = $derived(_args?.perPage ?? 1000)

	let label = $derived(filters?.label ?? null)
	let worker = $derived(filters?.worker ?? null)
	let success = $derived(filters?.status ?? null)
	let showSkipped = $derived(filters?.show_skipped ?? false)
	let showSchedules = $derived(!filters?.job_trigger_kind?.includes('!schedule'))
	let showFutureJobs = $derived(filters?.show_future_jobs ?? true)
	let resultFilter = $derived(filters?.result)
	let jobTriggerKind = $derived(filters?.job_trigger_kind ?? null)
	let schedulePath = $derived(filters?.schedule_path ?? null)
	let jobKindsCat = $derived(filters?.job_kinds ?? null)
	let allWorkspaces = $derived(filters?.all_workspaces ?? false)
	let allowWildcards = $derived(_allowWildcards(filters))
	let concurrencyKey = $derived(filters?.concurrency_key)
	let tag = $derived(filters?.tag)
	let user = $derived(filters?.user)
	let folder = $derived(filters?.folder)
	let path = $derived(filters?.path)
	let argFilter = $derived(filters?.arg)

	let queue_count: Tweened<number> | undefined = $state()
	let suspended_count: Tweened<number> | undefined = $state()
	let loading = $state(false)
	let lastFetchWentToEnd = $state(false)

	let completedJobs: CompletedJob[] | undefined = $state()
	let externalJobs: Job[] | undefined = $state()
	let extendedJobs: ExtendedJobs | undefined = $state()
	let jobs: Job[] | undefined = $state()

	let intervalId: ReturnType<typeof setInterval> | undefined = $state()
	let sync = true

	function onParamChanges() {
		resetJobs()
		let promise = loadJobsIntern(true)
		promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
			sendUserToast(
				'Loading jobs is taking longer than expected...',
				true,
				perPage > 25 && onSetPerPage
					? [{ label: 'Reduce to 25 items per page', callback: () => onSetPerPage(25) }]
					: []
			)
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) {
				return CancelablePromiseUtils.pure<void>(undefined)
			}
			return CancelablePromiseUtils.err(e)
		})
		return promise
	}

	let loadingFetch = false

	async function loadExtraJobs(): Promise<void> {
		if (jobs && jobs.length > 0) {
			let minQueueTs: string | undefined = undefined
			let minCompletedTs: string | undefined = undefined

			let cursor = 0

			while (jobs && cursor < jobs?.length) {
				cursor++
				const job = jobs[jobs.length - 1 - cursor]
				if (job.type == 'CompletedJob') {
					minCompletedTs = job.completed_at
					break
				} else if (job.type == 'QueuedJob' && minQueueTs == undefined) {
					minQueueTs = job.created_at
				}
			}

			const ts = minCompletedTs ?? minQueueTs

			if (!ts) {
				sendUserToast('No jobs to load from')
				lastFetchWentToEnd = false
				return
			}
			// const minCreated = lastJob?.created_at
			const minCreated = new Date(new Date(ts).getTime() - 1).toISOString()

			const minTs = timeframe?.computeMinMax().minTs ?? null
			let olderJobs = await fetchJobs(minCreated, minTs, undefined)
			jobs = updateWithNewJobs(olderJobs ?? [], jobs ?? [])
			computeCompletedJobs()
			lastFetchWentToEnd = olderJobs?.length < perPage
		} else {
			lastFetchWentToEnd = false
		}
	}

	function fetchJobs(
		completedBefore: string | null,
		completedAfter: string | null,
		createdAfterQueue: string | undefined
	): CancelablePromise<Job[]> {
		if (_args.skip) return CancelablePromiseUtils.pure<Job[]>([])
		loadingFetch = true
		let scriptPathStart = folder == null || folder === '' ? undefined : `f/${folder}/`
		let scriptPathExact = path == null || path === '' ? undefined : path
		let promise = JobService.listJobs({
			workspace: currentWorkspace,
			completedBefore: completedBefore ?? undefined,
			completedAfter: completedAfter ?? undefined,
			createdAfterQueue,
			schedulePath: schedulePath ?? undefined,
			scriptPathExact,
			createdBy: user == null || user === '' ? undefined : user,
			scriptPathStart: scriptPathStart,
			jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running:
				success == 'running' || success == 'suspended'
					? true
					: success == 'waiting'
						? false
						: undefined,
			isSkipped: showSkipped ? undefined : false,
			// isFlowStep: jobKindsCat != 'all' ? false : undefined,
			hasNullParent: jobKindsCat != 'all' ? true : undefined,
			label: label == null || label === '' ? undefined : label,
			tag: tag == null || tag === '' ? undefined : tag,
			worker: worker == null || worker === '' ? undefined : worker,
			isNotSchedule: showSchedules == false ? true : undefined,
			suspended: success == 'waiting' ? false : success == 'suspended' ? true : undefined,
			scheduledForBeforeNow:
				showFutureJobs == false || success == 'waiting' || success == 'suspended'
					? true
					: undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			triggerKind: jobTriggerKind ?? undefined,
			allWorkspaces: allWorkspaces ? true : undefined,
			perPage,
			allowWildcards: allowWildcards ? true : undefined
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.err(e)
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return CancelablePromiseUtils.pure<Job[]>([])
		})
		CancelablePromiseUtils.pipe(promise, () => {
			loadingFetch = false
		})
		return promise
	}

	function fetchExtendedJobs(
		concurrencyKey: string | null,
		createdBeforeQueue: string | null,
		completedAfter: string | null
	): CancelablePromise<ExtendedJobs> {
		if (_args.skip)
			return CancelablePromiseUtils.pure<ExtendedJobs>({ jobs: [], obscured_jobs: [] })
		loadingFetch = true
		let promise = ConcurrencyGroupsService.listExtendedJobs({
			rowLimit: perPage,
			concurrencyKey: concurrencyKey == null || concurrencyKey == '' ? undefined : concurrencyKey,
			workspace: currentWorkspace,
			completedAfter: completedAfter ?? undefined,
			createdBeforeQueue: createdBeforeQueue ?? undefined,
			// createdOrStartedBefore: startedBefore,
			// createdOrStartedAfter: startedAfter,
			// createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
			schedulePath: schedulePath ?? undefined,
			scriptPathExact: path == null || path === '' ? undefined : path,
			createdBy: user == null || user === '' ? undefined : user,
			scriptPathStart: folder == null || folder === '' ? undefined : `f/${folder}/`,
			jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running: success == 'running' ? true : undefined,
			isSkipped: showSkipped ? undefined : false,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			label: label == null || label === '' ? undefined : label,
			tag: tag == null || tag === '' ? undefined : tag,
			isNotSchedule: showSchedules == false ? true : undefined,
			scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			triggerKind: jobTriggerKind ?? undefined,
			allWorkspaces: allWorkspaces ? true : undefined,
			perPage,
			allowWildcards
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return CancelablePromiseUtils.pure({ jobs: [], obscured_jobs: [] } as ExtendedJobs)
		})
		promise = CancelablePromiseUtils.pipe(promise, () => {
			loadingFetch = false
		})
		return promise
	}

	async function loadJobs(reset: boolean, shouldGetCount?: boolean): Promise<void> {
		if (reset) resetJobs()
		await loadJobsIntern(shouldGetCount)
	}

	function resetJobs() {
		jobs = undefined
		completedJobs = undefined
		externalJobs = undefined
		extendedJobs = undefined
		lastFetchWentToEnd = false
		intervalId && clearInterval(intervalId)
		intervalId = setInterval(syncer, refreshRate)
	}
	function loadJobsIntern(shouldGetCount?: boolean): CancelablePromise<void> {
		const { minTs, maxTs } = timeframe?.computeMinMax() ?? { minTs: null, maxTs: null }
		if (shouldGetCount) {
			getCount()
		}
		loading = true
		// Extend MinTs to fetch jobs mefore minTs and show a correct concurrency graph
		// TODO: when an ended_at column is created on the completed_job table,
		// lookback won't be needed anymore (just filter ended_at > minTs instead
		const extendedMinTs = subtractDaysFromDateString(minTs, lookback)

		if (concurrencyKey == null || concurrencyKey === '') {
			return CancelablePromiseUtils.map(fetchJobs(maxTs, null, extendedMinTs), (newJobs) => {
				extendedJobs = { jobs: newJobs, obscured_jobs: [] } as ExtendedJobs

				// Filter on minTs here and not in the backend
				// to get enough data for the concurrency graph
				jobs = sortMinDate(minTs, newJobs)
				externalJobs = []
				computeCompletedJobs()
				loading = false
			})
		} else {
			return CancelablePromiseUtils.map(
				fetchExtendedJobs(concurrencyKey, maxTs, extendedMinTs ?? null),
				(newExtendedJobs) => {
					extendedJobs = newExtendedJobs
					const newJobs = newExtendedJobs.jobs
					const newExternalJobs = newExtendedJobs.obscured_jobs

					// Filter on minTs here and not in the backend
					// to get enough data for the concurrency graph
					if (minTs != undefined) {
						const minDate = new Date(minTs)
						jobs = newJobs.filter((x) =>
							x.started_at
								? new Date(x.started_at) > minDate
								: x.created_at
									? new Date(x.created_at) > minDate
									: false
						)
						externalJobs = computeExternalJobs(
							newExternalJobs.filter((x) => x.started_at && new Date(x.started_at) > minDate)
						)
					} else {
						jobs = newJobs
						externalJobs = computeExternalJobs(newExternalJobs)
					}
					computeCompletedJobs()
					loading = false
				}
			)
		}
	}

	async function getCount() {
		if (_args.skip) return
		const { database_length, suspended } = await JobService.getQueueCount({
			workspace: currentWorkspace,
			allWorkspaces
		})

		if (queue_count) {
			queue_count.set(database_length)
		} else {
			queue_count = tweened(database_length, { duration: 1000 })
		}
		if (suspended_count) {
			suspended_count.set(suspended ?? 0)
		} else {
			suspended_count = tweened(suspended ?? 0, { duration: 1000 })
		}
	}

	let lastQueueTs: string | undefined = undefined

	async function syncer() {
		if (loadingFetch) {
			return
		}
		if (sync) {
			if (syncQueuedRunsCount) {
				getCount()
			}

			const { minTs, maxTs } = timeframe?.computeMinMax() ?? { minTs: null, maxTs: null }
			if (jobs) {
				if (success == 'running') {
					loadJobsIntern(false)
				} else {
					let minQueueCreatedAt: string | undefined = undefined
					let completedTs: string | null = null

					let cursor = 0

					if (minTs == undefined) {
						while (cursor < jobs.length) {
							const cjob = jobs[cursor]
							if (cjob.type == 'QueuedJob') {
								minQueueCreatedAt = cjob.created_at
							} else if (cjob.type == 'CompletedJob' && completedTs == undefined) {
								completedTs = new Date(cjob.completed_at!).toISOString()
							}
							cursor++
						}
					}

					let queueTs: string | undefined
					if (minQueueCreatedAt) {
						const queueTs = new Date(minQueueCreatedAt).toISOString()
						lastQueueTs = queueTs
					} else {
						queueTs = lastQueueTs
					}

					loading = true
					let newJobs: Job[]
					if (concurrencyKey == null || concurrencyKey === '') {
						newJobs = await fetchJobs(maxTs, minTs ?? completedTs, queueTs)
					} else {
						// Obscured jobs have no ids, so we have to do the full request
						extendedJobs = await fetchExtendedJobs(concurrencyKey, maxTs, minTs ?? completedTs)
						externalJobs = computeExternalJobs(extendedJobs.obscured_jobs)

						// Filter on minTs here and not in the backend
						// to get enough data for the concurrency graph
						newJobs = sortMinDate(minTs ?? completedTs, extendedJobs.jobs)
					}
					if (newJobs && newJobs.length > 0 && jobs) {
						jobs = updateWithNewJobs(jobs, newJobs)
						if (concurrencyKey == null || concurrencyKey === '') {
							if (!extendedJobs) {
								extendedJobs = { jobs: jobs, obscured_jobs: [] } as ExtendedJobs
							} else {
								extendedJobs.jobs = updateWithNewJobs(extendedJobs.jobs, newJobs)
								extendedJobs = extendedJobs
							}
							externalJobs = []
						}
						computeCompletedJobs()
					}
					loading = false
				}
			}
		}
	}

	function updateWithNewJobs(jobs: Job[], newJobs: Job[]) {
		const oldJobs = jobs?.map((x) => x.id)
		let ret = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
		newJobs
			.filter((x) => oldJobs.includes(x.id))
			.forEach((x) => (ret![ret?.findIndex((y) => y.id == x.id)!] = x))
		return ret
	}

	function sortMinDate(minTs: string | null, jobs: Job[]) {
		if (minTs) {
			const minDate = new Date(minTs)
			return jobs.filter((x) =>
				x.started_at
					? new Date(x.started_at) > minDate
					: x.created_at
						? new Date(x.created_at) > minDate
						: false
			)
		} else {
			return jobs
		}
	}

	function computeCompletedJobs() {
		completedJobs =
			jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []
	}

	function onVisibilityChange() {
		if (document.hidden) {
			sync = false
		} else {
			sync = true
		}
	}

	function computeExternalJobs(obscuredJobs: ObscuredJob[]) {
		return obscuredJobs.map(
			(x) =>
				({
					type: x.typ,
					started_at: x.started_at,
					running: x.started_at != undefined,
					id: '-',
					script_path: '-',
					created_by: '-',
					created_at: '-',
					success: false,
					canceled: false,
					is_flow_step: false,
					is_skipped: false,
					visible_to_owner: false,
					email: '-',
					permissioned_as: '-',
					tag: '-',
					job_kind: 'script',
					duration_ms: x.duration_ms
				}) as Job
		)
	}

	onMount(() => {
		document.addEventListener('visibilitychange', onVisibilityChange)

		return () => {
			window.removeEventListener('visibilitychange', onVisibilityChange)
		}
	})

	onDestroy(() => {
		sync = false
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
	$effect(() => {
		Object.keys(filters ?? {}).map((k) => filters?.[k as keyof RunsFilterInstance])
		currentWorkspace
		lookback
		timeframe
		perPage
		showSchedules
		showFutureJobs
		let p = untrack(() => onParamChanges())
		return () => p.cancel()
	})
	$effect(() => {
		;[autoRefresh, refreshRate]
		untrack(() => {
			if (!intervalId && autoRefresh) {
				intervalId = setInterval(syncer, refreshRate)
			}
		})
	})
	$effect(() => {
		autoRefresh
		untrack(() => {
			if (intervalId && !autoRefresh) {
				clearInterval(intervalId)
				intervalId = undefined
			}
		})
	})

	return {
		loadExtraJobs,
		loadJobs,
		get queue_count() {
			return queue_count
		},
		get suspended_count() {
			return suspended_count
		},
		get loading() {
			return loading
		},
		get completedJobs() {
			return completedJobs
		},
		get externalJobs() {
			return externalJobs
		},
		get extendedJobs() {
			return extendedJobs
		},
		get jobs() {
			return jobs
		},
		get lastFetchWentToEnd() {
			return lastFetchWentToEnd
		}
	}
}
