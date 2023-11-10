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
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import RunChart from '$lib/components/RunChart.svelte'

	import JobPreview from '$lib/components/runs/JobPreview.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { RefreshCw } from 'lucide-svelte'
	import CalendarPicker from '$lib/components/common/calendarPicker/CalendarPicker.svelte'

	import RunsTable from '$lib/components/runs/RunsTable.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RunsFilter from '$lib/components/runs/RunsFilter.svelte'
	import MobileFilters from '$lib/components/runs/MobileFilters.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { tweened, type Tweened } from 'svelte/motion'
	import { goto } from '$app/navigation'
	import RunsQueue from '$lib/components/runs/RunsQueue.svelte'

	let jobs: Job[] | undefined
	let intervalId: NodeJS.Timer | undefined
	let selectedId: string | undefined = undefined

	// All Filters
	// Filter by
	let path: string | null = $page.params.path
	let user: string | null = $page.url.searchParams.get('user')
	let folder: string | null = $page.url.searchParams.get('folder')
	// Rest of filters handled by RunsFilter
	let success: 'running' | 'success' | 'failure' | undefined = ($page.url.searchParams.get(
		'success'
	) ?? undefined) as 'running' | 'success' | 'failure' | undefined
	let isSkipped: boolean | undefined =
		$page.url.searchParams.get('is_skipped') != undefined
			? $page.url.searchParams.get('is_skipped') == 'true'
			: false

	let argFilter: any = $page.url.searchParams.get('arg')
		? JSON.parse(decodeURIComponent($page.url.searchParams.get('arg') ?? '{}'))
		: undefined
	let resultFilter: any = $page.url.searchParams.get('result')
		? JSON.parse(decodeURIComponent($page.url.searchParams.get('result') ?? '{}'))
		: undefined

	let schedulePath = $page.url.searchParams.get('schedule_path') ?? undefined
	let jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'

	// Handled on the main page
	let minTs = $page.url.searchParams.get('min_ts') ?? undefined
	let maxTs = $page.url.searchParams.get('max_ts') ?? undefined

	// This reactive statement is used to sync the url with the current state of the filters
	$: {
		let searchParams = new URLSearchParams()

		user && searchParams.set('user', user)
		folder && searchParams.set('folder', folder)

		if (success !== undefined) {
			searchParams.set('success', success.toString())
		}

		if (isSkipped) {
			searchParams.set('is_skipped', isSkipped.toString())
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

	let queue_count: Tweened<number> | undefined = undefined

	$: jobKinds = computeJobKinds(jobKindsCat)

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == 'all') {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES},${CompletedJob.job_kind.APPDEPENDENCIES},${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW},${CompletedJob.job_kind.SCRIPT_HUB}`
		} else if (jobKindsCat == 'dependencies') {
			return `${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES},${CompletedJob.job_kind.APPDEPENDENCIES}`
		} else if (jobKindsCat == 'previews') {
			return `${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW}`
		} else {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW}`
		}
	}

	$: ($workspaceStore && loadJobs()) ||
		(path && success && isSkipped && jobKinds && user && folder && minTs && maxTs)

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

	let loading: boolean = false
	async function loadJobs(): Promise<void> {
		loading = true
		getCount()
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
		const qc = (await JobService.getQueueCount({ workspace: $workspaceStore! })).database_length
		if (queue_count) {
			queue_count.set(qc)
		} else {
			queue_count = tweened(qc, { duration: 1000 })
		}
	}

	async function syncer() {
		getCount()
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
				}
				loading = false
			}
		}
	}

	let sync = true
	let mounted: boolean = false

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

	onMount(() => {
		mounted = true
		loadPaths()
		loadUsernames()
		loadFolders()
		intervalId = setInterval(syncer, 5000)

		document.addEventListener('visibilitychange', () => {
			if (document.hidden) {
				sync = false
			} else {
				sync = true
			}
		})

		window.addEventListener('popstate', updateFiltersFromURL)
		return () => {
			window.removeEventListener('popstate', updateFiltersFromURL)
		}
	})

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})

	$: if (mounted && !intervalId && autoRefresh) {
		intervalId = setInterval(syncer, 5000)
	}

	$: if (mounted && intervalId && !autoRefresh) {
		clearInterval(intervalId)
		intervalId = undefined
	}

	let paths: string[] = []
	let usernames: string[] = []
	let folders: string[] = []

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

	let completedJobs: CompletedJob[] | undefined = undefined

	function computeCompletedJobs() {
		completedJobs =
			jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []
	}

	let argError = ''
	let resultError = ''
	let filterTimeout: NodeJS.Timeout | undefined = undefined

	function reloadLogsWithoutFilterError() {
		if (resultError == '' && argError == '') {
			filterTimeout && clearTimeout(filterTimeout)
			filterTimeout = setTimeout(() => {
				loadJobs()
			}, 2000)
		}
	}

	const manualDates = [
		{
			label: 'Last 1000 runs',
			setMinMax: () => {
				minTs = undefined
				maxTs = undefined
			}
		},
		{
			label: 'Within 30 seconds',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last minute',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 5 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 5 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 30 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 24 hours',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last 7 days',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Within last month',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		}
	]

	let selectedManualDate = 0
	let autoRefresh: boolean = true
	let runDrawer: Drawer
	let cancelAllJobs = false
</script>

<ConfirmationModal
	title="Confirm cancelling all jobs"
	confirmationText="Cancel all jobs"
	open={cancelAllJobs}
	on:confirmed={async () => {
		cancelAllJobs = false
		let uuids = await JobService.cancelAll({ workspace: $workspaceStore ?? '' })
		loadJobs()
		sendUserToast(`Canceled ${uuids.length} jobs`)
	}}
	on:canceled={() => {
		cancelAllJobs = false
	}}
/>

<Drawer bind:this={runDrawer}>
	<DrawerContent title="Run details" on:close={runDrawer.closeDrawer}>
		{#if selectedId}
			<JobPreview blankLink id={selectedId} />
		{/if}
	</DrawerContent>
</Drawer>

<div class="w-full h-screen hidden md:block">
	<div class="px-2">
		<div class="flex items-center space-x-2 flex-row justify-between">
			<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1 items-center">
				<h1 class="!text-2xl font-semibold leading-6 tracking-tight"> Runs </h1>

				<Tooltip
					documentationLink="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs"
				>
					All past and schedule executions of scripts and flows, including previews. You only see
					your own runs or runs of groups you belong to unless you are an admin.
				</Tooltip>
			</div>
			<div class="hidden xl:block">
				<RunsFilter
					bind:isSkipped
					bind:user
					bind:folder
					bind:path
					bind:success
					bind:argFilter
					bind:resultFilter
					bind:argError
					bind:resultError
					bind:jobKindsCat
					on:change={reloadLogsWithoutFilterError}
					{usernames}
					{folders}
					{paths}
				/>
			</div>
			<div class="xl:hidden">
				<MobileFilters>
					<svelte:fragment slot="filters">
						<span class="text-xs font-semibold leading-6">Filters</span>
						<RunsFilter
							bind:isSkipped
							{paths}
							{usernames}
							{folders}
							bind:jobKindsCat
							bind:folder
							bind:path
							bind:user
							bind:success
							bind:argFilter
							bind:resultFilter
							bind:argError
							bind:resultError
							on:change={reloadLogsWithoutFilterError}
						/>
					</svelte:fragment>
				</MobileFilters>
			</div>
		</div>
	</div>

	<div class="p-2 w-full">
		<RunChart
			maxIsNow={maxTs == undefined}
			jobs={completedJobs}
			on:zoom={async (e) => {
				minTs = e.detail.min.toISOString()
				maxTs = e.detail.max.toISOString()
			}}
		/>
	</div>
	<div class="flex flex-col gap-1 md:flex-row w-full p-4">
		<div class="flex gap-2 grow mb-2">
			<RunsQueue {queue_count} />
			<div class="flex"
				><Button
					size="xs"
					color="light"
					variant="contained"
					title="Require to be an admin. Cancel all jobs in queue"
					disabled={!$userStore?.is_admin && !$superadmin}
					on:click={async () => (cancelAllJobs = true)}>Cancel All</Button
				></div
			>
		</div>
		<div class="flex flex-row gap-1 w-full max-w-xl">
			<div class="relative w-full">
				<div class="flex gap-1 relative w-full">
					<span class="text-xs absolute -top-4">Min datetime</span>

					<input type="text" value={minTs ?? 'zoom x axis to set min (drag with ctrl)'} disabled />

					<CalendarPicker
						date={minTs}
						label="Min datetimes"
						on:change={async ({ detail }) => {
							minTs = new Date(detail).toISOString()
						}}
					/>
				</div>
			</div>
			<div class="relative w-full">
				<div class="flex gap-1 relative w-full">
					<span class="text-xs absolute -top-4">Max datetime</span>
					<input type="text" value={maxTs ?? 'zoom x axis to set max'} disabled />
					<CalendarPicker
						date={maxTs}
						label="Max datetimes"
						on:change={async ({ detail }) => {
							maxTs = new Date(detail).toISOString()
						}}
					/>
				</div>
			</div>
		</div>
		<div class="flex flex-row gap-2 items-center">
			<Button
				size="xs"
				color="light"
				variant="border"
				on:click={() => {
					minTs = undefined
					maxTs = undefined

					autoRefresh = true
					jobs = undefined
					completedJobs = undefined
					selectedManualDate = 0
					selectedId = undefined
					loadJobs()
				}}
			>
				Reset
			</Button>
			<Button
				color="light"
				size="xs"
				wrapperClasses="border rounded-md"
				on:click={() => {
					manualDates[selectedManualDate].setMinMax()
					loadJobs()
				}}
				dropdownItems={[
					...manualDates.map((d, i) => ({
						label: d.label,
						onClick: () => {
							selectedManualDate = i
							d.setMinMax()
							loadJobs()
						}
					}))
				]}
				startIcon={{ icon: RefreshCw, classes: loading ? 'animate-spin' : '' }}
			>
				{manualDates[selectedManualDate].label}
			</Button>

			<Toggle
				size="xs"
				bind:checked={autoRefresh}
				options={{ right: 'Auto-refresh' }}
				textClass="whitespace-nowrap"
			/>
		</div>
	</div>

	<SplitPanesWrapper>
		<Splitpanes>
			<Pane size={60} minSize={40}>
				{#if jobs}
					<RunsTable
						{jobs}
						bind:selectedId
						on:filterByPath={(e) => {
							user = null
							folder = null
							path = e.detail
						}}
						on:filterByUser={(e) => {
							path = null
							folder = null
							user = e.detail
						}}
						on:filterByFolder={(e) => {
							path = null
							user = null
							folder = e.detail
						}}
					/>
				{:else}
					<div class="gap-1 flex flex-col">
						{#each new Array(8) as _}
							<Skeleton layout={[[3]]} />
						{/each}
					</div>
				{/if}
			</Pane>
			<Pane size={40} minSize={15} class="border-t">
				{#if selectedId}
					<JobPreview id={selectedId} />
				{:else}
					<div class="text-xs m-4">No job selected</div>
				{/if}
			</Pane>
		</Splitpanes>
	</SplitPanesWrapper>
</div>

<div class="md:hidden h-[calc(100vh-48px)]">
	<SplitPanesWrapper>
		<Splitpanes horizontal>
			<Pane size={50} minSize={20}>
				<div class="px-2">
					<div class="flex items-center space-x-2 flex-row justify-between">
						<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1">
							<h1 class="!text-2xl font-semibold leading-6 tracking-tight"> Runs </h1>

							<Tooltip
								light
								documentationLink="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs"
								scale={0.9}
								wrapperClass="flex items-center"
							>
								All past and schedule executions of scripts and flows, including previews. You only
								see your own runs or runs of groups you belong to unless you are an admin.
							</Tooltip>
						</div>
						<div class="hidden xl:block">
							<RunsFilter
								bind:isSkipped
								bind:user
								bind:folder
								bind:path
								bind:success
								bind:argFilter
								bind:resultFilter
								bind:argError
								bind:resultError
								bind:jobKindsCat
								on:change={reloadLogsWithoutFilterError}
								{usernames}
								{folders}
								{paths}
							/>
						</div>
						<div class="xl:hidden">
							<MobileFilters>
								<svelte:fragment slot="filters">
									<span class="text-xs font-semibold leading-6">Filters</span>
									<RunsFilter
										bind:isSkipped
										{paths}
										{usernames}
										{folders}
										bind:jobKindsCat
										bind:folder
										bind:path
										bind:user
										bind:success
										bind:argFilter
										bind:resultFilter
										bind:argError
										bind:resultError
										on:change={reloadLogsWithoutFilterError}
									/>
								</svelte:fragment>
							</MobileFilters>
						</div>
					</div>
				</div>
				<div class="p-2 w-full">
					<RunChart
						maxIsNow={maxTs == undefined}
						jobs={completedJobs}
						on:zoom={async (e) => {
							minTs = e.detail.min.toISOString()
							maxTs = e.detail.max.toISOString()
						}}
					/>
				</div>
				<div class="flex flex-col gap-1 md:flex-row w-full p-4">
					<div class="flex gap-2 grow mb-2">
						{#if queue_count}
							<RunsQueue {queue_count} />
						{/if}
						<div class="flex"
							><Button
								size="xs"
								color="light"
								variant="contained"
								title="Require to be an admin. Cancel all jobs in queue"
								disabled={!$userStore?.is_admin && !$superadmin}
								on:click={async () => (cancelAllJobs = true)}>Cancel All</Button
							></div
						>
					</div>
					<div class="flex flex-row gap-1 w-full max-w-xl">
						<div class="relative w-full">
							<div class="flex gap-1 relative w-full">
								<span class="text-xs absolute -top-4">Min datetime</span>

								<input
									type="text"
									value={minTs ?? 'zoom x axis to set min (drag with ctrl)'}
									disabled
								/>

								<CalendarPicker
									date={minTs}
									label="Min datetimes"
									on:change={async ({ detail }) => {
										minTs = new Date(detail).toISOString()
									}}
								/>
							</div>
						</div>
						<div class="relative w-full">
							<div class="flex gap-1 relative w-full">
								<span class="text-xs absolute -top-4">Max datetime</span>
								<input type="text" value={maxTs ?? 'zoom x axis to set max'} disabled />
								<CalendarPicker
									date={maxTs}
									label="Max datetimes"
									on:change={async ({ detail }) => {
										maxTs = new Date(detail).toISOString()
									}}
								/>
							</div>
						</div>
					</div>
					<div class="flex flex-row gap-2 items-center">
						<Button
							size="xs"
							color="light"
							variant="border"
							on:click={() => {
								minTs = undefined
								maxTs = undefined

								autoRefresh = true
								jobs = undefined
								completedJobs = undefined
								selectedManualDate = 0
								selectedId = undefined
								loadJobs()
							}}
						>
							Reset
						</Button>
						<Button
							color="light"
							size="xs"
							wrapperClasses="border rounded-md"
							on:click={() => {
								manualDates[selectedManualDate].setMinMax()
								loadJobs()
							}}
							dropdownItems={[
								...manualDates.map((d, i) => ({
									label: d.label,
									onClick: () => {
										selectedManualDate = i
										d.setMinMax()
										loadJobs()
									}
								}))
							]}
						>
							<div class="flex flex-row items-center gap-2">
								<RefreshCw size={14} class={loading ? 'animate-spin' : ''} />
								{manualDates[selectedManualDate].label}
							</div>
						</Button>

						<Toggle
							size="xs"
							bind:checked={autoRefresh}
							options={{ right: 'Auto-refresh' }}
							textClass="whitespace-nowrap"
						/>
					</div>
				</div>
			</Pane>
			<Pane size={50} minSize={20}>
				<div class="overflow-y-hidden h-full">
					<RunsTable
						{jobs}
						bind:selectedId
						on:select={() => {
							runDrawer.openDrawer()
						}}
						on:filterByPath={(e) => {
							user = null
							folder = null
							path = e.detail
						}}
						on:filterByUser={(e) => {
							path = null
							folder = null
							user = e.detail
						}}
						on:filterByFolder={(e) => {
							path = null
							user = null
							folder = e.detail
						}}
					/>
				</div>
			</Pane>
		</Splitpanes>
	</SplitPanesWrapper>
</div>
