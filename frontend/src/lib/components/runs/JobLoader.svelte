<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import {
		JobService,
		Job,
		CompletedJob,
		ScriptService,
		FlowService,
		UserService,
		FolderService
	} from '$lib/gen'

	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	import { tweened, type Tweened } from 'svelte/motion'
	import { goto } from '$app/navigation'
	import { forLater } from '$lib/forLater'

	export let jobs: Job[] | undefined
	export let user: string | null
	export let folder: string | null
	export let path: string | null
	export let success: 'success' | 'failure' | 'running' | undefined = undefined
	export let isSkipped: boolean = false
	export let hideSchedules: boolean = false
	export let argFilter: string | undefined
	export let resultFilter: string | undefined = undefined
	export let schedulePath: string | undefined = undefined
	export let jobKindsCat: string
	export let minTs: string | undefined = undefined
	export let maxTs: string | undefined = undefined
	export let jobKinds: string = ''
	export let queue_count: Tweened<number> | undefined = undefined
	export let autoRefresh: boolean = true
	export let paths: string[] = []
	export let usernames: string[] = []
	export let folders: string[] = []
	export let completedJobs: CompletedJob[] | undefined = undefined
	export let argError = ''
	export let resultError = ''
	export let loading: boolean = false
	export let synUrl: boolean = true
	export let refreshRate = 5000
	export let syncQueuedRunsCount: boolean = true

	let mounted: boolean = false
	let intervalId: NodeJS.Timeout | undefined
	let sync = true

	// This reactive statement is used to sync the url with the current state of the filters
	$: if (synUrl) {
		let searchParams = new URLSearchParams()

		user && searchParams.set('user', user)
		folder && searchParams.set('folder', folder)

		if (success !== undefined) {
			searchParams.set('success', success.toString())
		}

		if (isSkipped) {
			searchParams.set('is_skipped', isSkipped.toString())
		}

		if (hideSchedules) {
			searchParams.set('hide_scheduled', hideSchedules.toString())
		}

		// ArgFilter is an object. Encode it to a string
		argFilter && searchParams.set('arg', encodeURIComponent(JSON.stringify(argFilter)))
		resultFilter && searchParams.set('result', encodeURIComponent(JSON.stringify(resultFilter)))
		schedulePath && searchParams.set('schedule_path', schedulePath)

		jobKindsCat != 'runs' && searchParams.set('job_kinds', jobKindsCat)

		minTs && searchParams.set('min_ts', minTs)
		maxTs && searchParams.set('max_ts', maxTs)

		let newPath = path ? `/${path}` : '/'
		let newUrl = `/runs${newPath}?${searchParams.toString()}`

		goto(newUrl)
	}

	$: jobKinds = computeJobKinds(jobKindsCat)
	$: ($workspaceStore && loadJobs()) ||
		(path && success && isSkipped && jobKinds && user && folder && minTs && maxTs && hideSchedules)

	$: if (mounted && !intervalId && autoRefresh) {
		intervalId = setInterval(syncer, refreshRate)
	}

	$: if (mounted && intervalId && !autoRefresh) {
		clearInterval(intervalId)
		intervalId = undefined
	}

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == 'all') {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES},${CompletedJob.job_kind.APPDEPENDENCIES},${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW},${CompletedJob.job_kind.SCRIPT_HUB},${CompletedJob.job_kind.DEPLOYMENTCALLBACK},${CompletedJob.job_kind.SINGLESCRIPTFLOW}`
		} else if (jobKindsCat == 'dependencies') {
			return `${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES},${CompletedJob.job_kind.APPDEPENDENCIES}`
		} else if (jobKindsCat == 'previews') {
			return `${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW}`
		} else if (jobKindsCat == 'deploymentcallbacks') {
			return `${CompletedJob.job_kind.DEPLOYMENTCALLBACK}`
		} else {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.SINGLESCRIPTFLOW}`
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
			isSkipped,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined
		})
	}

	export async function loadJobs(shouldGetCount?: boolean): Promise<void> {
		if (shouldGetCount) {
			getCount()
		}

		loading = true
		try {
			jobs = await fetchJobs(maxTs, minTs)

			computeCompletedJobs()

			if (hideSchedules && !schedulePath) {
				jobs = jobs.filter(
					(job) => !(job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for))
				)
			}
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			console.error(JSON.stringify(err))
		}
		loading = false
	}

	async function getCount() {
		const qc = (await JobService.getQueueCount({ workspace: $workspaceStore! })).database_length
		if (queue_count) {
			queue_count.set(qc)
		} else {
			queue_count = tweened(qc, { duration: 1000 })
		}
	}

	async function syncer() {
		if (syncQueuedRunsCount) {
			getCount()
		}

		if (sync && jobs && maxTs == undefined) {
			if (success == 'running') {
				loadJobs()
			} else {
				let ts: string | undefined = undefined
				let cursor = 0

				while (cursor < jobs.length && minTs == undefined) {
					let invCursor = jobs.length - 1 - cursor
					let isQueuedJob =
						cursor == jobs?.length - 1 || jobs[invCursor].type == Job.type.QUEUED_JOB
					if (isQueuedJob) {
						if (cursor > 0) {
							const date = new Date(jobs[invCursor + 1]?.created_at!)
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

					if (hideSchedules && !schedulePath) {
						jobs = jobs.filter(
							(job) =>
								!(job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for))
						)
					}
				}

				loading = false
			}
		}
	}

	function updateFiltersFromURL() {
		path = $page.params.path
		user = $page.url.searchParams.get('user')
		folder = $page.url.searchParams.get('folder')
		success = ($page.url.searchParams.get('success') ?? undefined) as
			| 'success'
			| 'failure'
			| 'running'
			| undefined
		isSkipped =
			$page.url.searchParams.get('is_skipped') != undefined
				? $page.url.searchParams.get('is_skipped') == 'true'
				: false

		hideSchedules =
			$page.url.searchParams.get('hide_scheduled') != undefined
				? $page.url.searchParams.get('hide_scheduled') == 'true'
				: false

		argFilter = $page.url.searchParams.get('arg')
			? JSON.parse(decodeURIComponent($page.url.searchParams.get('arg') ?? '{}'))
			: undefined
		resultFilter = $page.url.searchParams.get('result')
			? JSON.parse(decodeURIComponent($page.url.searchParams.get('result') ?? '{}'))
			: undefined

		schedulePath = $page.url.searchParams.get('schedule_path') ?? undefined
		jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'

		// Handled on the main page
		minTs = $page.url.searchParams.get('min_ts') ?? undefined
	}

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	async function loadFolders(): Promise<void> {
		folders = await FolderService.listFolders({
			workspace: $workspaceStore!
		}).then((x) => x.map((y) => y.name))
	}

	async function loadPaths() {
		const npaths_scripts = await ScriptService.listScriptPaths({ workspace: $workspaceStore ?? '' })
		const npaths_flows = await FlowService.listFlowPaths({ workspace: $workspaceStore ?? '' })
		paths = npaths_scripts.concat(npaths_flows).sort()
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
		mounted = true
		loadPaths()
		loadUsernames()
		loadFolders()

		intervalId = setInterval(syncer, refreshRate)

		document.addEventListener('visibilitychange', onVisibilityChange)

		window.addEventListener('popstate', updateFiltersFromURL)
		return () => {
			window.removeEventListener('popstate', updateFiltersFromURL)
			window.removeEventListener('visibilitychange', onVisibilityChange)
		}
	})

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
</script>
