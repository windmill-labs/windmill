<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import {
		JobService,
		type Job,
		type CompletedJob,
		type ConcurrencyIntervals,
		ConcurrencyGroupsService
	} from '$lib/gen'

	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	import { tweened, type Tweened } from 'svelte/motion'

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
	export let concurrencyIntervals: ConcurrencyIntervals | undefined = undefined
	export let argError = ''
	export let resultError = ''
	export let loading: boolean = false
	export let refreshRate = 5000
	export let syncQueuedRunsCount: boolean = true
	export let allWorkspaces: boolean = false
	export let computeMinAndMax: (() => { minTs: string; maxTs: string } | undefined) | undefined

	let intervalId: NodeJS.Timeout | undefined
	let sync = true
	let concurrencyKeyMap: Map<string, string> = new Map<string, string>()

	$: jobKinds = computeJobKinds(jobKindsCat)
	$: ($workspaceStore && loadJobsIntern(true)) ||
		(path &&
			label &&
			success &&
			isSkipped != undefined &&
			jobKinds &&
			concurrencyKey &&
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
		startedAfter: string | undefined
	): Promise<Job[]> {
		return JobService.listJobs({
			workspace: $workspaceStore!,
			createdOrStartedBefore: startedBefore,
			createdOrStartedAfter: startedAfter,
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

	async function fetchConcurrencyIntervals(
		concurrencyKey: string | null,
		startedBefore: string | undefined,
		startedAfter: string | undefined
	): Promise<ConcurrencyIntervals> {
		return ConcurrencyGroupsService.getConcurrencyIntervals({
			rowLimit: 1000,
			concurrencyKey: concurrencyKey == null || concurrencyKey == '' ? undefined : concurrencyKey,
			workspace: $workspaceStore!,
			createdOrStartedBefore: startedBefore,
			createdOrStartedAfter: startedAfter,
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
			concurrencyIntervals = undefined
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
			concurrencyIntervals = await fetchConcurrencyIntervals(concurrencyKey, maxTs, undefined)
			updateConcurrencyKeyMap()
			computeExternalJobs(minTs)
			let j = await fetchJobs(maxTs, minTs)
			jobs = await filterJobsByConcurrencyKey(j, minTs)
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
					concurrencyIntervals = await fetchConcurrencyIntervals(concurrencyKey, maxTs, undefined)
					updateConcurrencyKeyMap()
					computeExternalJobs(minTs)
					let newJobs = await fetchJobs(maxTs, minTs ?? ts)
					newJobs = (await filterJobsByConcurrencyKey(newJobs, minTs)) ?? []
					if (newJobs && newJobs.length > 0 && jobs) {
						const oldJobs = jobs?.map((x) => x.id)
						jobs = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
						newJobs
							.filter((x) => oldJobs.includes(x.id))
							.forEach((x) => (jobs![jobs?.findIndex((y) => y.id == x.id)!] = x))
						jobs = jobs
						computeCompletedJobs()
					}
					loading = false
				}
			}
		}
	}

	function computeCompletedJobs() {
		completedJobs =
			jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []
	}

	async function filterJobsByConcurrencyKey(jobs: Job[] | undefined, minTs: string | undefined) {
		if (concurrencyKey == null || concurrencyKey === '' || jobs == undefined || jobs.length === 0)
			return jobs

		let minDate = minTs ? new Date(minTs) : undefined

		return jobs.filter((x) => concurrencyKeyMap.get(x.id) === concurrencyKey && (!minDate || (x.started_at && minDate < new Date(x.started_at))))
	}

	function onVisibilityChange() {
		if (document.hidden) {
			sync = false
		} else {
			sync = true
		}
	}

	function updateConcurrencyKeyMap() {
		for (const vec of [concurrencyIntervals?.running_jobs, concurrencyIntervals?.completed_jobs]) {
			if (vec == undefined) continue
			for (const interval of vec) {
				if (
					interval.job_id &&
					interval.concurrency_key &&
					concurrencyKeyMap.get(interval.job_id) == undefined
				) {
					concurrencyKeyMap.set(interval.job_id, interval.concurrency_key)
				}
			}
		}
	}

	function computeExternalJobs(minTs: string | undefined) {
		let minDate = minTs ? new Date(minTs) : undefined
		let externalQueued = concurrencyIntervals?.running_jobs
			.filter((x) => !x.job_id && (!minDate || (x.started_at && minDate < new Date(x.started_at))))
			.map(
				(x) =>
					({
						id: '-',
						type: 'QueuedJob',
						started_at: x.started_at,
						running: x.started_at != undefined,
						script_path: '-'
					} as Job)
			)
		let externalCompleted = concurrencyIntervals?.completed_jobs
			.filter((x) => !x.job_id && (!minDate || (x.started_at && minDate < new Date(x.started_at))))
			.map(
				(x) =>
					({
						type: 'CompletedJob',
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
						job_kind: 'flow',
						duration_ms:
							x.ended_at && x.started_at
								? new Date(x.ended_at).getTime() - new Date(x.started_at).getTime()
								: 0
					} as Job)
			)
		let ret: Job[] = []
		if (externalQueued) {
			ret = ret.concat(externalQueued)
		}
		if (externalCompleted) {
			ret = ret.concat(externalCompleted)
		}
		externalJobs = ret
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
