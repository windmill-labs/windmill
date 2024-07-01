<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
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

	export let jobs: Job[] | undefined
	export let user: string | null
	export let label: string | null = null
	export let folder: string | null
	export let path: string | null
	export let success: 'success' | 'failure' | 'running' | undefined = undefined
	export let isSkipped: boolean = false
	export let showSchedules: boolean = true
	export let showFutureJobs: boolean = true
	export let argFilter: string | undefined
	export let resultFilter: string | undefined = undefined
	export let schedulePath: string | undefined = undefined
	export let jobKindsCat: string
	export let minTs: string | undefined = undefined
	export let maxTs: string | undefined = undefined
	export let jobKinds: string = ''
	export let queue_count: Tweened<number> | undefined = undefined
	export let autoRefresh: boolean = true
	export let completedJobs: CompletedJob[] | undefined = undefined
	export let externalJobs: Job[] | undefined = undefined
	export let concurrencyKey: string | null
	export let extendedJobs: ExtendedJobs | undefined = undefined
	export let argError = ''
	export let resultError = ''
	export let loading: boolean = false
	export let refreshRate = 5000
	export let syncQueuedRunsCount: boolean = true
	export let allWorkspaces: boolean = false
	export let computeMinAndMax: (() => { minTs: string; maxTs: string } | undefined) | undefined
	export let lookback: number = 0

	let intervalId: NodeJS.Timeout | undefined
	let sync = true

	$: jobKinds = computeJobKinds(jobKindsCat)
	$: ($workspaceStore && loadJobsIntern(true)) ||
		(path &&
			label &&
			success &&
			isSkipped != undefined &&
			jobKinds &&
			concurrencyKey &&
			lookback &&
			user &&
			folder &&
			showFutureJobs != undefined &&
			showSchedules != undefined &&
			allWorkspaces != undefined &&
			argFilter != undefined &&
			resultFilter != undefined)

	$: if (!intervalId && autoRefresh) {
		intervalId = setInterval(syncer, refreshRate)
	}

	$: if (intervalId && !autoRefresh) {
		clearInterval(intervalId)
		intervalId = undefined
	}

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == 'all') {
			let kinds: CompletedJob['job_kind'][] = [
				'script',
				'flow',
				'dependencies',
				'flowdependencies',
				'appdependencies',
				'preview',
				'flowpreview',
				'script_hub'
			]
			return kinds.join(',')
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
		} else {
			let kinds: CompletedJob['job_kind'][] = ['script', 'flow']
			return kinds.join(',')
		}
	}

	async function fetchJobs(
		startedBefore: string | undefined,
		startedAfter: string | undefined,
		startedAfterCompletedJobs: string | undefined,
	): Promise<Job[]> {
		console.log('fetching jobs', startedAfter, startedBefore)
		return JobService.listJobs({
			workspace: $workspaceStore!,
			createdOrStartedBefore: startedBefore,
			createdOrStartedAfter: startedAfter,
			createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
			schedulePath,
			scriptPathExact: path === null || path === '' ? undefined : path,
			createdBy: user === null || user === '' ? undefined : user,
			scriptPathStart: folder === null || folder === '' ? undefined : `f/${folder}/`,
			jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running: success == 'running' ? true : undefined,
			isSkipped: isSkipped ? undefined : false,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			label: label === null || label === '' ? undefined : label,
			isNotSchedule: showSchedules == false ? true : undefined,
			scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			allWorkspaces: allWorkspaces ? true : undefined
		})
	}

	async function fetchExtendedJobs(
		concurrencyKey: string | null,
		startedBefore: string | undefined,
		startedAfter: string | undefined,
		startedAfterCompletedJobs: string | undefined
	): Promise<ExtendedJobs> {
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
			jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running: success == 'running' ? true : undefined,
			isSkipped: isSkipped ? undefined : false,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			label: label === null || label === '' ? undefined : label,
			isNotSchedule: showSchedules == false ? true : undefined,
			scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			allWorkspaces: allWorkspaces ? true : undefined
		})
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
			jobs = undefined
			completedJobs = undefined
			externalJobs = undefined
			extendedJobs = undefined
			intervalId && clearInterval(intervalId)
			intervalId = setInterval(syncer, refreshRate)
		}
		await loadJobsIntern(shouldGetCount)
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
				let newJobs = await fetchJobs(maxTs, undefined, extendedMinTs)
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
		const qc = (await JobService.getQueueCount({ workspace: $workspaceStore!, allWorkspaces }))
			.database_length
		if (queue_count) {
			queue_count.set(qc)
		} else {
			queue_count = tweened(qc, { duration: 1000 })
		}
	}

	async function syncer() {
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
					const extendedMinTs = subtractDaysFromDateString(minTs, lookback)
					let newJobs: Job[]
					if (concurrencyKey == null || concurrencyKey === '') {
						newJobs = await fetchJobs(maxTs, extendedMinTs ?? ts, undefined)
					} else {
						// Obscured jobs have no ids, so we have to do the full request
						extendedJobs = await fetchExtendedJobs(concurrencyKey, maxTs, undefined, extendedMinTs ?? ts)
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
				} as Job)
		)
	}

	onMount(() => {
		document.addEventListener('visibilitychange', onVisibilityChange)

		return () => {
			window.removeEventListener('visibilitychange', onVisibilityChange)
		}
	})

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
</script>
