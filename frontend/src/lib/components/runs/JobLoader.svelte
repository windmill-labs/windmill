<script lang="ts">
	import { onDestroy, onMount, untrack } from 'svelte'
	import {
		JobService,
		type Job,
		type CompletedJob,
		type ExtendedJobs,
		ConcurrencyGroupsService,
		type ObscuredJob
	} from '$lib/gen'

	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	import { tweened, type Tweened } from 'svelte/motion'
	import { subtractDaysFromDateString } from '$lib/utils'

	interface Props {
		jobs: Job[] | undefined
		user: string | null
		label?: string | null
		worker?: string | null
		folder: string | null
		path: string | null
		success?: 'success' | 'suspended' | 'waiting' | 'failure' | 'running' | undefined
		isSkipped?: boolean
		showSchedules?: boolean
		showFutureJobs?: boolean
		argFilter: string | undefined
		resultFilter?: string | undefined
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
		extendedJobs?: ExtendedJobs | undefined
		argError?: string
		resultError?: string
		loading?: boolean
		refreshRate?: number
		syncQueuedRunsCount?: boolean
		allWorkspaces?: boolean
		computeMinAndMax: (() => { minTs: string; maxTs: string | undefined } | undefined) | undefined
		lookback?: number
		perPage?: number | undefined
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
		isSkipped = false,
		showSchedules = true,
		showFutureJobs = true,
		argFilter,
		resultFilter = undefined,
		schedulePath = undefined,
		jobKindsCat = undefined,
		minTs = $bindable(undefined),
		maxTs = $bindable(undefined),
		jobKinds = $bindable(''),
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
		perPage = undefined,
		allowWildcards = false
	}: Props = $props()
	let intervalId: NodeJS.Timeout | undefined = $state()
	let sync = true

	function onParamChanges() {
		resetJobs()
		loadJobsIntern(true)
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
			let kinds: CompletedJob['job_kind'][] = ['script', 'flow', 'singlescriptflow']
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
			const lastJob = jobs[jobs.length - 1]
			// const minCreated = lastJob?.created_at
			const minCreated = new Date(new Date(lastJob.created_at!).getTime() - 1).toISOString()

			let olderJobs = await fetchJobs(undefined, minTs, undefined, minCreated)
			jobs = jobs.concat(olderJobs)
			computeCompletedJobs()
			return olderJobs?.length < 1000
		}
		return false
	}

	async function fetchJobs(
		startedBefore: string | undefined,
		startedAfter: string | undefined,
		startedAfterCompletedJobs: string | undefined,
		createdBefore: string | undefined
	): Promise<Job[]> {
		loadingFetch = true
		try {
			let scriptPathStart = folder === null || folder === '' ? undefined : `f/${folder}/`
			let scriptPathExact = path === null || path === '' ? undefined : path
			return JobService.listJobs({
				workspace: $workspaceStore!,
				createdOrStartedBefore: startedBefore,
				createdOrStartedAfter: startedAfter,
				createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
				schedulePath,
				scriptPathExact,
				createdBefore,
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
				isSkipped: isSkipped ? undefined : false,
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
					argFilter && argFilter != '{}' && argFilter != '' && argError == ''
						? argFilter
						: undefined,
				result:
					resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
						? resultFilter
						: undefined,
				allWorkspaces: allWorkspaces ? true : undefined,
				perPage,
				allowWildcards
			})
		} catch (e) {
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return []
		} finally {
			loadingFetch = false
		}
	}

	async function fetchExtendedJobs(
		concurrencyKey: string | null,
		startedBefore: string | undefined,
		startedAfter: string | undefined,
		startedAfterCompletedJobs: string | undefined
	): Promise<ExtendedJobs> {
		loadingFetch = true
		try {
			return ConcurrencyGroupsService.listExtendedJobs({
				rowLimit: 1000,
				concurrencyKey: concurrencyKey == null || concurrencyKey == '' ? undefined : concurrencyKey,
				workspace: $workspaceStore!,
				createdOrStartedBefore: startedBefore,
				createdOrStartedAfter: startedAfter,
				createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
				schedulePath,
				scriptPathExact: path === null || path === '' ? undefined : path,
				createdBy: user === null || user === '' ? undefined : user,
				scriptPathStart: folder === null || folder === '' ? undefined : `f/${folder}/`,
				jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
				success: success == 'success' ? true : success == 'failure' ? false : undefined,
				running: success == 'running' ? true : undefined,
				isSkipped: isSkipped ? undefined : false,
				isFlowStep: jobKindsCat != 'all' ? false : undefined,
				label: label === null || label === '' ? undefined : label,
				tag: tag === null || tag === '' ? undefined : tag,
				isNotSchedule: showSchedules == false ? true : undefined,
				scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
				args:
					argFilter && argFilter != '{}' && argFilter != '' && argError == ''
						? argFilter
						: undefined,
				result:
					resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
						? resultFilter
						: undefined,
				allWorkspaces: allWorkspaces ? true : undefined,
				perPage,
				allowWildcards
			})
		} catch (e) {
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return {
				jobs: [],
				obscured_jobs: []
			}
		} finally {
			loadingFetch = false
		}
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
	async function loadJobsIntern(shouldGetCount?: boolean): Promise<void> {
		if (shouldGetCount) {
			getCount()
		}
		loading = true
		try {
			// Extend MinTs to fetch jobs mefore minTs and show a correct concurrency graph
			// TODO: when an ended_at column is created on the completed_job table,
			// lookback won't be needed anymore (just filter ended_at > minTs instead
			const extendedMinTs = subtractDaysFromDateString(minTs, lookback)
			if (concurrencyKey == null || concurrencyKey === '') {
				let newJobs = await fetchJobs(maxTs, undefined, extendedMinTs, undefined)
				extendedJobs = { jobs: newJobs, obscured_jobs: [] } as ExtendedJobs

				// Filter on minTs here and not in the backend
				// to get enough data for the concurrency graph
				jobs = sortMinDate(minTs, newJobs)
				externalJobs = []
			} else {
				extendedJobs = await fetchExtendedJobs(concurrencyKey, maxTs, undefined, extendedMinTs)
				const newJobs = extendedJobs.jobs
				const newExternalJobs = extendedJobs.obscured_jobs

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
			}
			computeCompletedJobs()
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			console.error(JSON.stringify(err))
		}
		loading = false
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
					let ts: string | undefined = undefined
					let cursor = 0

					while (cursor < jobs.length && minTs == undefined) {
						let invCursor = jobs.length - 1 - cursor
						let isQueuedJob = invCursor == 0 || jobs[invCursor].type == 'QueuedJob'
						if (isQueuedJob) {
							if (cursor > 0) {
								let inc = invCursor == 0 && jobs[invCursor].type == 'CompletedJob' ? 0 : 1
								const date = new Date(
									jobs[invCursor + inc]?.started_at ?? jobs[invCursor + inc]?.created_at!
								)
								date.setMilliseconds(date.getMilliseconds() + 1)
								ts = date.toISOString()
							}
							break
						}
						cursor++
					}

					loading = true
					let newJobs: Job[]
					if (concurrencyKey == null || concurrencyKey === '') {
						newJobs = await fetchJobs(maxTs, minTs ?? ts, undefined, undefined)
					} else {
						// Obscured jobs have no ids, so we have to do the full request
						extendedJobs = await fetchExtendedJobs(concurrencyKey, maxTs, undefined, minTs ?? ts)
						externalJobs = computeExternalJobs(extendedJobs.obscured_jobs)

						// Filter on minTs here and not in the backend
						// to get enough data for the concurrency graph
						newJobs = sortMinDate(minTs ?? ts, extendedJobs.jobs)
					}
					if (newJobs && newJobs.length > 0 && jobs) {
						jobs = updateWithNewJobs(jobs, newJobs)
						jobs = jobs
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
		;($workspaceStore ||
			(path &&
				label &&
				success &&
				worker &&
				isSkipped != undefined &&
				jobKinds &&
				concurrencyKey &&
				tag &&
				lookback &&
				user &&
				folder &&
				allowWildcards &&
				schedulePath != undefined &&
				showFutureJobs != undefined &&
				showSchedules != undefined &&
				allWorkspaces != undefined &&
				argFilter != undefined &&
				resultFilter != undefined)) &&
			untrack(() => onParamChanges())
	})
	$effect(() => {
		if (!intervalId && autoRefresh) {
			intervalId = setInterval(
				untrack(() => syncer),
				refreshRate
			)
		}
	})
	$effect(() => {
		if (intervalId && !autoRefresh) {
			clearInterval(intervalId)
			intervalId = undefined
		}
	})
</script>
