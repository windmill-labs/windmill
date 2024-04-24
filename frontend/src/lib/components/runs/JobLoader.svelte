<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import { JobService, type Job, type CompletedJob } from '$lib/gen'

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
	export let showSchedules: boolean = false
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
	export let argError = ''
	export let resultError = ''
	export let loading: boolean = false
	export let refreshRate = 5000
	export let syncQueuedRunsCount: boolean = true
	export let allWorkspaces: boolean = false
	export let computeMinAndMax: (() => { minTs: string; maxTs: string } | undefined) | undefined

	let intervalId: NodeJS.Timeout | undefined
	let sync = true

	$: jobKinds = computeJobKinds(jobKindsCat)
	$: ($workspaceStore && loadJobsIntern(true)) ||
		(path &&
			label &&
			success &&
			isSkipped != undefined &&
			jobKinds &&
			user &&
			folder &&
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
			jobs = await fetchJobs(maxTs, minTs)
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
					const newJobs = await fetchJobs(maxTs, minTs ?? ts)
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

	function onVisibilityChange() {
		if (document.hidden) {
			sync = false
		} else {
			sync = true
		}
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
