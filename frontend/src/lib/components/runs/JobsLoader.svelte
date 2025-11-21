<script lang="ts">
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
	import { workspaceStore } from '$lib/stores'

	import { tweened, type Tweened } from 'svelte/motion'
	import { subtractDaysFromDateString } from '$lib/utils'
	import type { JobTriggerType } from '../triggers/utils'
	import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'

	interface Props {
		jobs: Job[] | undefined
		user: string | null
		label?: string | null
		worker?: string | null
		folder: string | null
		path: string | null
		success?: 'success' | 'suspended' | 'waiting' | 'failure' | 'running' | undefined
		showSchedules?: boolean
		showFutureJobs?: boolean
		argFilter: string | undefined
		resultFilter?: string | undefined
		jobTriggerKind?: JobTriggerType | undefined
		schedulePath?: string | undefined
		jobKindsCat?: string | undefined
		minTs?: string | undefined
		maxTs?: string | undefined
		jobKinds?: string
		queue_count?: Tweened<number> | undefined
		suspended_count?: Tweened<number> | undefined
		autoRefresh?: boolean
		completedJobs?: CompletedJob[] | undefined
		externalJobs?: Job[] | undefined
		concurrencyKey: string | null
		tag: string | null
		showSkipped?: boolean
		extendedJobs?: ExtendedJobs | undefined
		argError?: string
		resultError?: string
		loading?: boolean
		refreshRate?: number
		syncQueuedRunsCount?: boolean
		allWorkspaces?: boolean
		computeMinAndMax: (() => { minTs: string; maxTs: string | undefined } | undefined) | undefined
		lookback?: number
		perPage?: number
		allowWildcards?: boolean
	}

	let {
		jobs = $bindable(),
		user,
		label = null,
		worker = null,
		folder,
		path,
		success = undefined,
		showSkipped = false,
		showSchedules = true,
		showFutureJobs = true,
		argFilter,
		resultFilter = undefined,
		jobTriggerKind = undefined,
		schedulePath = undefined,
		jobKindsCat = undefined,
		minTs = $bindable(undefined),
		maxTs = $bindable(undefined),
		jobKinds = $bindable(undefined),
		queue_count = $bindable(undefined),
		suspended_count = $bindable(undefined),
		autoRefresh = true,
		completedJobs = $bindable(undefined),
		externalJobs = $bindable(undefined),
		concurrencyKey,
		tag,
		extendedJobs = $bindable(undefined),
		argError = '',
		resultError = '',
		loading = $bindable(false),
		refreshRate = 5000,
		syncQueuedRunsCount = true,
		allWorkspaces = false,
		computeMinAndMax,
		lookback = 0,
		perPage = $bindable(1000),
		allowWildcards = false
	}: Props = $props()
	let intervalId: number | undefined = $state()
	let sync = true

	function onParamChanges() {
		resetJobs()
		let promise = loadJobsIntern(true)
		promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
			sendUserToast(
				'Loading jobs is taking longer than expected...',
				true,
				perPage > 25
					? [{ label: 'Reduce to 25 items per page', callback: () => (perPage = 25) }]
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

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == undefined && jobKinds != undefined) {
			return jobKinds
		}
		if (jobKindsCat == 'all') {
			return ''
		} else if (jobKindsCat == 'dependencies') {
			let kinds: CompletedJob['job_kind'][] = [
				'dependencies',
				'flowdependencies',
				'appdependencies'
			]
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

	let loadingFetch = false

	export async function loadExtraJobs(): Promise<boolean> {
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
				return false
			}
			// const minCreated = lastJob?.created_at
			const minCreated = new Date(new Date(ts).getTime() - 1).toISOString()

			let olderJobs = await fetchJobs(minCreated, minTs, undefined)
			jobs = jobs?.concat(olderJobs)
			computeCompletedJobs()
			return olderJobs?.length < perPage
		}
		return false
	}

	function fetchJobs(
		completedBefore: string | undefined,
		completedAfter: string | undefined,
		createdAfterQueue: string | undefined
	): CancelablePromise<Job[]> {
		loadingFetch = true
		let scriptPathStart = folder === null || folder === '' ? undefined : `f/${folder}/`
		let scriptPathExact = path === null || path === '' ? undefined : path
		let promise = JobService.listJobs({
			workspace: $workspaceStore!,
			completedBefore,
			completedAfter,
			createdAfterQueue,
			schedulePath,
			scriptPathExact,
			createdBy: user === null || user === '' ? undefined : user,
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
			label: label === null || label === '' ? undefined : label,
			tag: tag === null || tag === '' ? undefined : tag,
			worker: worker === null || worker === '' ? undefined : worker,
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
			triggerKind: jobTriggerKind,
			allWorkspaces: allWorkspaces ? true : undefined,
			perPage,
			allowWildcards: allowWildcards ? true : undefined
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.err(e)
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return CancelablePromiseUtils.pure([] as Job[])
		})
		CancelablePromiseUtils.pipe(promise, () => {
			loadingFetch = false
		})
		return promise
	}

	function fetchExtendedJobs(
		concurrencyKey: string | null,
		createdBeforeQueue: string | undefined,
		completedAfter: string | undefined
	): CancelablePromise<ExtendedJobs> {
		loadingFetch = true
		let promise = ConcurrencyGroupsService.listExtendedJobs({
			rowLimit: perPage,
			concurrencyKey: concurrencyKey == null || concurrencyKey == '' ? undefined : concurrencyKey,
			workspace: $workspaceStore!,
			completedAfter,
			createdBeforeQueue: createdBeforeQueue,
			// createdOrStartedBefore: startedBefore,
			// createdOrStartedAfter: startedAfter,
			// createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
			schedulePath,
			scriptPathExact: path === null || path === '' ? undefined : path,
			createdBy: user === null || user === '' ? undefined : user,
			scriptPathStart: folder === null || folder === '' ? undefined : `f/${folder}/`,
			jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running: success == 'running' ? true : undefined,
			isSkipped: showSkipped ? undefined : false,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			label: label === null || label === '' ? undefined : label,
			tag: tag === null || tag === '' ? undefined : tag,
			isNotSchedule: showSchedules == false ? true : undefined,
			scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			triggerKind: jobTriggerKind,
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

	export async function loadJobs(
		nMinTs: string | undefined,
		nMaxTs: string | undefined,
		reset: boolean,
		shouldGetCount?: boolean
	): Promise<void> {
		minTs = nMinTs
		maxTs = nMaxTs
		if (reset) {
			resetJobs()
		}
		await loadJobsIntern(shouldGetCount)
	}

	function resetJobs() {
		jobs = undefined
		completedJobs = undefined
		externalJobs = undefined
		extendedJobs = undefined
		intervalId && clearInterval(intervalId)
		intervalId = setInterval(syncer, refreshRate)
	}
	function loadJobsIntern(shouldGetCount?: boolean): CancelablePromise<void> {
		if (shouldGetCount) {
			getCount()
		}
		loading = true
		// Extend MinTs to fetch jobs mefore minTs and show a correct concurrency graph
		// TODO: when an ended_at column is created on the completed_job table,
		// lookback won't be needed anymore (just filter ended_at > minTs instead
		const extendedMinTs = subtractDaysFromDateString(minTs, lookback)

		if (concurrencyKey == null || concurrencyKey === '') {
			return CancelablePromiseUtils.map(fetchJobs(maxTs, undefined, extendedMinTs), (newJobs) => {
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
				fetchExtendedJobs(concurrencyKey, maxTs, extendedMinTs),
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
		const { database_length, suspended } = await JobService.getQueueCount({
			workspace: $workspaceStore!,
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
		if (success == 'waiting') {
			minTs = undefined
			maxTs = undefined
		}
		if (loadingFetch) {
			return
		}
		if (sync) {
			if (syncQueuedRunsCount) {
				getCount()
			}

			if (computeMinAndMax) {
				const ts = computeMinAndMax()
				if (ts) {
					minTs = ts.minTs
					maxTs = ts.maxTs
					if (maxTs != undefined) {
						loadJobsIntern(false)
					}
				}
			}

			if (jobs && maxTs == undefined) {
				if (success == 'running') {
					loadJobsIntern(false)
				} else {
					let minQueueCreatedAt: string | undefined = undefined
					let completedTs: string | undefined = undefined

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

	function sortMinDate(minTs: string | undefined, jobs: Job[]) {
		if (minTs != undefined) {
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
		jobKinds = computeJobKinds(jobKindsCat)
	})
	$effect(() => {
		;[
			$workspaceStore,
			path,
			label,
			success,
			worker,
			showSkipped,
			jobKinds,
			concurrencyKey,
			tag,
			lookback,
			user,
			folder,
			allowWildcards,
			schedulePath,
			showFutureJobs,
			showSchedules,
			allWorkspaces,
			argFilter,
			resultFilter,
			jobTriggerKind,
			perPage
		]
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
</script>
