<script lang="ts">
	import {
		JobService,
		type Job,
		type CompletedJob,
		UserService,
		FolderService,
		ScriptService,
		FlowService,
		type ExtendedJobs
	} from '$lib/gen'

	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import RunChart from '$lib/components/RunChart.svelte'

	import JobPreview from '$lib/components/runs/JobPreview.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import CalendarPicker from '$lib/components/common/calendarPicker/CalendarPicker.svelte'

	import RunsTable from '$lib/components/runs/RunsTable.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RunsFilter from '$lib/components/runs/RunsFilter.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { Tweened } from 'svelte/motion'
	import RunsQueue from '$lib/components/runs/RunsQueue.svelte'
	import { twMerge } from 'tailwind-merge'
	import ManuelDatePicker from '$lib/components/runs/ManuelDatePicker.svelte'
	import JobLoader from '$lib/components/runs/JobLoader.svelte'
	import { AlertTriangle, Calendar, Check, ChevronDown, Clock, X } from 'lucide-svelte'
	import ConcurrentJobsChart from '$lib/components/ConcurrentJobsChart.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { goto } from '$app/navigation'
	import { base } from '$app/paths'
	import { isJobCancelable } from '$lib/utils'

	let jobs: Job[] | undefined
	let selectedIds: string[] = []
	let selectedWorkspace: string | undefined = undefined

	// All Filters
	// Filter by
	let path: string | null = $page.params.path
	let user: string | null = $page.url.searchParams.get('user')
	let folder: string | null = $page.url.searchParams.get('folder')
	let label: string | null = $page.url.searchParams.get('label')
	let concurrencyKey: string | null = $page.url.searchParams.get('concurrency_key')
	let tag: string | null = $page.url.searchParams.get('tag')
	// Rest of filters handled by RunsFilter
	let success: 'running' | 'suspended' | 'waiting' | 'success' | 'failure' | undefined =
		($page.url.searchParams.get('success') ?? undefined) as
			| 'running'
			| 'success'
			| 'failure'
			| undefined
	let isSkipped: boolean | undefined =
		$page.url.searchParams.get('is_skipped') != undefined
			? $page.url.searchParams.get('is_skipped') == 'true'
			: false

	let showSchedules: boolean =
		$page.url.searchParams.get('show_schedules') != undefined
			? $page.url.searchParams.get('show_schedules') == 'true'
			: localStorage.getItem('show_schedules_in_run') == 'false'
			? false
			: true
	let showFutureJobs: boolean =
		$page.url.searchParams.get('show_future_jobs') != undefined
			? $page.url.searchParams.get('show_future_jobs') == 'true'
			: localStorage.getItem('show_future_jobs') == 'false'
			? false
			: true

	let argFilter: any = $page.url.searchParams.get('arg')
		? JSON.parse(decodeURIComponent($page.url.searchParams.get('arg') ?? '{}'))
		: undefined
	let resultFilter: any = $page.url.searchParams.get('result')
		? JSON.parse(decodeURIComponent($page.url.searchParams.get('result') ?? '{}'))
		: undefined

	// Handled on the main page
	let minTs = $page.url.searchParams.get('min_ts') ?? undefined
	let maxTs = $page.url.searchParams.get('max_ts') ?? undefined
	let schedulePath = $page.url.searchParams.get('schedule_path') ?? undefined
	let jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'
	let allWorkspaces = $page.url.searchParams.get('all_workspaces') == 'true' ?? false
	let lastFetchWentToEnd = false

	function loadFromQuery() {
		path = $page.params.path
		user = $page.url.searchParams.get('user')
		folder = $page.url.searchParams.get('folder')
		label = $page.url.searchParams.get('label')
		concurrencyKey = $page.url.searchParams.get('concurrency_key')
		tag = $page.url.searchParams.get('tag')
		// Rest of filters handled by RunsFilter
		success = ($page.url.searchParams.get('success') ?? undefined) as
			| 'running'
			| 'success'
			| 'failure'
			| undefined
		isSkipped =
			$page.url.searchParams.get('is_skipped') != undefined
				? $page.url.searchParams.get('is_skipped') == 'true'
				: false

		showSchedules =
			$page.url.searchParams.get('show_schedules') != undefined
				? $page.url.searchParams.get('show_schedules') == 'true'
				: localStorage.getItem('show_schedules_in_run') == 'false'
				? false
				: true
		showFutureJobs =
			$page.url.searchParams.get('show_future_jobs') != undefined
				? $page.url.searchParams.get('show_future_jobs') == 'true'
				: localStorage.getItem('show_future_jobs') == 'false'
				? false
				: true

		argFilter = $page.url.searchParams.get('arg')
			? JSON.parse(decodeURIComponent($page.url.searchParams.get('arg') ?? '{}'))
			: undefined
		resultFilter = $page.url.searchParams.get('result')
			? JSON.parse(decodeURIComponent($page.url.searchParams.get('result') ?? '{}'))
			: undefined

		// Handled on the main page
		minTs = $page.url.searchParams.get('min_ts') ?? undefined
		maxTs = $page.url.searchParams.get('max_ts') ?? undefined
		schedulePath = $page.url.searchParams.get('schedule_path') ?? undefined
		jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'
		allWorkspaces = $page.url.searchParams.get('all_workspaces') == 'true' ?? false
	}

	let queue_count: Tweened<number> | undefined = undefined
	let suspended_count: Tweened<number> | undefined = undefined

	let jobKinds: string | undefined = undefined
	let loading: boolean = false
	let paths: string[] = []
	let usernames: string[] = []
	let folders: string[] = []
	let completedJobs: CompletedJob[] | undefined = undefined
	let extendedJobs: ExtendedJobs | undefined = undefined
	let argError = ''
	let resultError = ''
	let filterTimeout: NodeJS.Timeout | undefined = undefined
	let selectedManualDate = 0
	let autoRefresh: boolean = true
	let runDrawer: Drawer
	let isCancelingVisibleJobs = false
	let isCancelingFilteredJobs = false
	let lookback: number = 1

	let innerWidth = window.innerWidth
	let jobLoader: JobLoader | undefined = undefined
	let externalJobs: Job[] | undefined = undefined

	let graph: 'RunChart' | 'ConcurrencyChart' = typeOfChart($page.url.searchParams.get('graph'))
	let graphIsRunsChart: boolean = graph === 'RunChart'

	let manualDatePicker: ManuelDatePicker

	let runsTable: RunsTable

	$: (user ||
		label ||
		folder ||
		path ||
		success !== undefined ||
		isSkipped ||
		showSchedules ||
		showFutureJobs ||
		argFilter ||
		resultFilter ||
		schedulePath ||
		jobKindsCat ||
		concurrencyKey ||
		tag ||
		graph ||
		maxTs ||
		allWorkspaces ||
		$workspaceStore) &&
		setQuery(false)

	$: minTs || setQuery(true)

	function setQuery(replaceState: boolean) {
		let searchParams = new URLSearchParams()

		if (user) {
			searchParams.set('user', user)
		} else {
			searchParams.delete('user')
		}

		if (folder) {
			searchParams.set('folder', folder)
		} else {
			searchParams.delete('folder')
		}

		if (success !== undefined) {
			searchParams.set('success', success.toString())
		} else {
			searchParams.delete('success')
		}

		if (isSkipped) {
			searchParams.set('is_skipped', isSkipped.toString())
		} else {
			searchParams.delete('is_skipped')
		}

		if (showSchedules) {
			searchParams.set('show_schedules', showSchedules.toString())
		} else {
			searchParams.delete('show_schedules')
		}

		if (showFutureJobs) {
			searchParams.set('show_future_jobs', showFutureJobs.toString())
		} else {
			searchParams.delete('show_future_jobs')
		}

		if (allWorkspaces && $workspaceStore == 'admins') {
			searchParams.set('all_workspaces', allWorkspaces.toString())
			searchParams.set('workspace', 'admins')
		} else {
			searchParams.delete('all_workspaces')
		}

		// ArgFilter is an object. Encode it to a string
		if (argFilter) {
			searchParams.set('arg', encodeURIComponent(JSON.stringify(argFilter)))
		} else {
			searchParams.delete('arg')
		}

		if (resultFilter) {
			searchParams.set('result', encodeURIComponent(JSON.stringify(resultFilter)))
		} else {
			searchParams.delete('result')
		}
		if (schedulePath) {
			searchParams.set('schedule_path', schedulePath)
		} else {
			searchParams.delete('schedule_path')
		}
		if (jobKindsCat != 'runs') {
			searchParams.set('job_kinds', jobKindsCat)
		} else {
			searchParams.delete('job_kinds')
		}

		if (minTs) {
			searchParams.set('min_ts', minTs)
		} else {
			searchParams.delete('min_ts')
		}

		if (maxTs) {
			searchParams.set('max_ts', maxTs)
		} else {
			searchParams.delete('max_ts')
		}
		if (concurrencyKey) {
			searchParams.set('concurrency_key', concurrencyKey)
		} else {
			searchParams.delete('concurrency_key')
		}

		if (tag) {
			searchParams.set('tag', tag)
		} else {
			searchParams.delete('tag')
		}

		if (label) {
			searchParams.set('label', label)
		} else {
			searchParams.delete('label')
		}

		if (graph != 'RunChart') {
			searchParams.set('graph', graph)
		} else {
			searchParams.delete('graph')
		}

		let newPath = path ? `/${path}` : ''

		let newUrl = `${base}/runs${newPath}?${searchParams.toString()}`
		if (
			$page.url.searchParams.toString() != searchParams.toString() ||
			$page.url.pathname != newUrl.split('?')[0]
		) {
			// replaceState(newUrl.toString(), $page.state)
			goto(newUrl.toString(), { replaceState: replaceState })
		}
	}

	function reloadJobsWithoutFilterError() {
		if (resultError == '' && argError == '') {
			filterTimeout && clearTimeout(filterTimeout)
			filterTimeout = setTimeout(() => {
				jobLoader?.loadJobs(minTs, maxTs, true)
			}, 2000)
		}
	}

	function reset() {
		minTs = undefined
		maxTs = undefined
		jobs = undefined
		completedJobs = undefined
		lastFetchWentToEnd = false
		selectedManualDate = 0
		selectedIds = []
		jobIdsToCancel = []
		isSelectingJobsToCancel = false
		selectedWorkspace = undefined
		jobLoader?.loadJobs(minTs, maxTs, true)
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

	$: if ($workspaceStore) {
		loadUsernames()
		loadFolders()
		loadPaths()
	}

	function filterByPath(e: CustomEvent<string>) {
		path = e.detail
		user = null
		folder = null
		label = null
		concurrencyKey = null
		tag = null
		schedulePath = undefined
	}

	function filterByUser(e: CustomEvent<string>) {
		path = null
		folder = null
		user = e.detail
		label = null
		concurrencyKey = null
		tag = null
		schedulePath = undefined
	}

	function filterByFolder(e: CustomEvent<string>) {
		path = null
		user = null
		folder = e.detail
		label = null
		concurrencyKey = null
		tag = null
		schedulePath = undefined
	}

	function filterByLabel(e: CustomEvent<string>) {
		path = null
		user = null
		folder = null
		label = e.detail
		concurrencyKey = null
		tag = null
		schedulePath = undefined
	}

	function filterByConcurrencyKey(e: CustomEvent<string>) {
		path = null
		user = null
		folder = null
		label = null
		concurrencyKey = e.detail
		tag = null
		schedulePath = undefined
	}

	function filterByTag(e: CustomEvent<string>) {
		path = null
		user = null
		folder = null
		label = null
		concurrencyKey = null
		tag = e.detail
		schedulePath = undefined
	}

	function filterBySchedule(e: CustomEvent<string>) {
		path = null
		user = null
		folder = null
		label = null
		concurrencyKey = null
		tag = null
		schedulePath = e.detail
	}

	let calendarChangeTimeout: NodeJS.Timeout | undefined = undefined

	function typeOfChart(s: string | null): 'RunChart' | 'ConcurrencyChart' {
		switch (s) {
			case 'RunChart':
				return 'RunChart'
			case 'ConcurrencyChart':
				return 'ConcurrencyChart'
			default:
				return 'RunChart'
		}
	}

	let jobIdsToCancel: string[] = []
	let isSelectingJobsToCancel = false
	let fetchingFilteredJobs = false
	let selectedFiltersString: string | undefined = undefined

	async function cancelVisibleJobs() {
		isSelectingJobsToCancel = true
		selectedIds = jobs?.filter(isJobCancelable).map((j) => j.id) ?? []
		if (selectedIds.length === 0) {
			sendUserToast('There are no visible jobs that can be canceled', true)
		}
	}
	async function cancelFilteredJobs() {
		isCancelingFilteredJobs = true
		fetchingFilteredJobs = true
		const selectedFilters = {
			workspace: $workspaceStore ?? '',
			startedBefore: maxTs,
			startedAfter: minTs,
			schedulePath,
			scriptPathExact: path === null || path === '' ? undefined : path,
			createdBy: user === null || user === '' ? undefined : user,
			scriptPathStart: folder === null || folder === '' ? undefined : `f/${folder}/`,
			jobKinds,
			success: success == 'success' ? true : success == 'failure' ? false : undefined,
			running:
				success == 'running' || success == 'suspended'
					? true
					: success == 'waiting'
					? false
					: undefined,
			isSkipped: isSkipped ? undefined : false,
			// isFlowStep: jobKindsCat != 'all' ? false : undefined,
			hasNullParent:
				path != undefined || path != undefined || jobKindsCat != 'all' ? true : undefined,
			label: label === null || label === '' ? undefined : label,
			tag: tag === null || tag === '' ? undefined : tag,
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
			allWorkspaces: allWorkspaces ? true : undefined
		}

		selectedFiltersString = JSON.stringify(selectedFilters, null, 4)
		jobIdsToCancel = await JobService.listFilteredUuids(selectedFilters)
		fetchingFilteredJobs = false
	}

	async function cancelSelectedJobs() {
		jobIdsToCancel = selectedIds
		isCancelingVisibleJobs = true
	}

	function jobCountString(count: number) {
		return `${count} ${count == 1 ? 'job' : 'jobs'}`
	}

	function setLookback(lookbackInDays: number) {
		lookback = lookbackInDays
	}

	async function loadExtra() {
		if (jobLoader) {
			lastFetchWentToEnd = await jobLoader.loadExtraJobs()
			console.log(lastFetchWentToEnd)
		}
	}

	const warnJobLimitMsg =
		'The exact number of concurrent jobs at the beginning of the time range may be incorrect as only the last 1000 jobs are taken into account: a job that was started earlier than this limit will not be taken into account'

	$: warnJobLimit =
		graph === 'ConcurrencyChart' &&
		extendedJobs !== undefined &&
		extendedJobs.jobs.length + extendedJobs.obscured_jobs.length >= 1000

	function jobsFilter(f: 'waiting' | 'suspended') {
		path = null
		user = null
		folder = null
		label = null
		concurrencyKey = null
		schedulePath = undefined
		path = null
		tag = null
		if (success == f) {
			success = undefined
		} else {
			success = f
		}
		jobKindsCat = 'all'
	}
</script>

<JobLoader
	{allWorkspaces}
	bind:jobs
	{user}
	{folder}
	{path}
	{label}
	{success}
	{isSkipped}
	{argFilter}
	{resultFilter}
	{showSchedules}
	{showFutureJobs}
	{schedulePath}
	{jobKindsCat}
	computeMinAndMax={manualDatePicker?.computeMinMax}
	bind:minTs
	bind:maxTs
	{jobKinds}
	bind:queue_count
	bind:suspended_count
	{autoRefresh}
	bind:completedJobs
	bind:externalJobs
	bind:extendedJobs
	{concurrencyKey}
	{argError}
	{resultError}
	{tag}
	bind:loading
	bind:this={jobLoader}
	lookback={graphIsRunsChart ? 0 : lookback}
/>

<ConfirmationModal
	title={`Confirm cancelling all jobs correspoding to the selected filters (${jobIdsToCancel.length} jobs)`}
	confirmationText={`Cancel ${jobIdsToCancel.length} jobs that matched the filters`}
	open={isCancelingFilteredJobs}
	on:confirmed={async () => {
		isCancelingFilteredJobs = false
		let uuids = await JobService.cancelSelection({
			workspace: $workspaceStore ?? '',
			requestBody: jobIdsToCancel
		})
		jobIdsToCancel = []
		selectedIds = []
		jobLoader?.loadJobs(minTs, maxTs, true, true)
		sendUserToast(`Canceled ${uuids.length} jobs`)
		isSelectingJobsToCancel = false
	}}
	loading={fetchingFilteredJobs}
	on:canceled={() => {
		isCancelingFilteredJobs = false
	}}
>
	<pre>{selectedFiltersString}</pre>
</ConfirmationModal>

<ConfirmationModal
	title={`Confirm cancelling the selected jobs`}
	confirmationText={`Cancel ${jobIdsToCancel.length} jobs`}
	open={isCancelingVisibleJobs}
	on:confirmed={async () => {
		isCancelingVisibleJobs = false
		let uuids = await JobService.cancelSelection({
			workspace: $workspaceStore ?? '',
			requestBody: jobIdsToCancel
		})
		jobIdsToCancel = []
		selectedIds = []
		jobLoader?.loadJobs(minTs, maxTs, true, true)
		sendUserToast(`Canceled ${uuids.length} jobs`)
		isSelectingJobsToCancel = false
	}}
	on:canceled={() => {
		isCancelingVisibleJobs = false
	}}
/>

<Drawer bind:this={runDrawer}>
	<DrawerContent title="Run details" on:close={runDrawer.closeDrawer}>
		{#if selectedIds.length === 1}
			{#if selectedIds[0] === '-'}
				<div class="p-4">There is no information available for this job</div>
			{:else}
				<JobPreview blankLink id={selectedIds[0]} workspace={selectedWorkspace} />
			{/if}
		{/if}
	</DrawerContent>
</Drawer>

<svelte:window
	bind:innerWidth
	on:popstate={() => {
		reset()
		loadFromQuery()
	}}
/>

{#if innerWidth > 900}
	<div class="w-full h-screen">
		<div class="px-2">
			<div class="flex items-center space-x-2 flex-row justify-between">
				<div class="flex-col">
					<div class="flex flex-row flex-wrap justify-between py-2 my-4 px-4 gap-1 items-center">
						<h1
							class={twMerge(
								'!text-2xl font-semibold leading-6 tracking-tight',
								$userStore?.operator ? 'pl-10' : ''
							)}
						>
							Runs
						</h1>

						<Tooltip
							documentationLink="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs"
						>
							All past and schedule executions of scripts and flows, including previews. You only
							see your own runs or runs of groups you belong to unless you are an admin.
						</Tooltip>
					</div>
				</div>
				<RunsFilter
					bind:isSkipped
					bind:user
					bind:folder
					bind:label
					bind:concurrencyKey
					bind:tag
					bind:path
					bind:success
					bind:argFilter
					bind:resultFilter
					bind:argError
					bind:resultError
					bind:jobKindsCat
					bind:allWorkspaces
					bind:schedulePath
					on:change={reloadJobsWithoutFilterError}
					on:successChange={(e) => {
						if (e.detail == 'running' && maxTs != undefined) {
							maxTs = undefined
						}
					}}
					{usernames}
					{folders}
					{paths}
				/>
			</div>
		</div>

		<div class="p-2 w-full">
			<div class="relative z-10">
				<div class="absolute right-0 -mt-6">
					<div class="flex flex-row justify-between items-center">
						<ToggleButtonGroup
							bind:selected={graph}
							on:selected={() => {
								graphIsRunsChart = graph === 'RunChart'
							}}
						>
							<ToggleButton value="RunChart" label="Duration" />
							<ToggleButton
								value="ConcurrencyChart"
								label="Concurrency"
								icon={warnJobLimit ? AlertTriangle : undefined}
								tooltip={warnJobLimit ? warnJobLimitMsg : undefined}
							/>
						</ToggleButtonGroup>
					</div>
					{#if !graphIsRunsChart}
						<DropdownV2
							items={[
								{
									displayName: 'None',
									action: () => setLookback(0)
								},
								{
									displayName: '1 day',
									action: () => setLookback(1)
								},
								{
									displayName: '3 days',
									action: () => setLookback(3)
								},
								{
									displayName: '7 days',
									action: () => setLookback(7)
								}
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<ChevronDown class="w-5 h-5" />
									<span class="text-xs min-w-[5rem]">{lookback} days lookback</span>
									<Tooltip>
										How far behind the min datetime to start considering jobs for the concurrency
										graph. Change this value to include jobs started before the set time window for
										the computation of the graph
									</Tooltip>
								</div>
							</svelte:fragment>
						</DropdownV2>
					{/if}
				</div>
			</div>
			{#if graph === 'RunChart'}
				<RunChart
					{lastFetchWentToEnd}
					bind:selectedIds
					canSelect={!isSelectingJobsToCancel}
					minTimeSet={minTs}
					maxTimeSet={maxTs}
					maxIsNow={maxTs == undefined}
					on:loadExtra={loadExtra}
					jobs={completedJobs}
					on:zoom={async (e) => {
						minTs = e.detail.min.toISOString()
						maxTs = e.detail.max.toISOString()
						manualDatePicker?.resetChoice()
						jobLoader?.loadJobs(minTs, maxTs, true)
					}}
					on:pointClicked={(e) => {
						runsTable.scrollToRun(e.detail)
					}}
				/>
			{:else if graph === 'ConcurrencyChart'}
				<ConcurrentJobsChart
					minTimeSet={minTs}
					maxTimeSet={maxTs}
					maxIsNow={maxTs == undefined}
					{extendedJobs}
					on:zoom={async (e) => {
						minTs = e.detail.min.toISOString()
						maxTs = e.detail.max.toISOString()
						jobLoader?.loadJobs(minTs, maxTs, true)
					}}
				/>
			{/if}
		</div>
		<div class="flex flex-col gap-1 md:flex-row w-full p-4">
			<div class="flex gap-2 grow flex-row">
				<RunsQueue
					{success}
					{queue_count}
					{suspended_count}
					on:jobs_waiting={() => {
						jobsFilter('waiting')
					}}
					on:jobs_suspended={() => {
						jobsFilter('suspended')
					}}
				/>
				<div class="flex flex-row">
					{#if isSelectingJobsToCancel}
						<div class="mt-1 p-2 h-8 flex flex-row items-center gap-1">
							<Button
								startIcon={{ icon: X }}
								size="xs"
								color="gray"
								variant="contained"
								on:click={() => {
									isSelectingJobsToCancel = false
									selectedIds = []
								}}
							/>
							<Button
								disabled={selectedIds.length == 0}
								startIcon={{ icon: Check }}
								size="xs"
								color="red"
								variant="contained"
								on:click={cancelSelectedJobs}
							>
								Cancel {jobCountString(selectedIds.length)}
							</Button>
						</div>
					{:else if !$userStore?.is_admin && !$superadmin}
						<DropdownV2
							items={[
								{
									displayName: 'Select jobs to cancel',
									action: cancelVisibleJobs
								}
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<span class="text-xs min-w-[5rem]">Cancel jobs</span>
									<ChevronDown class="w-5 h-5" />
								</div>
							</svelte:fragment>
						</DropdownV2>
					{:else}
						<DropdownV2
							items={[
								{
									displayName: 'Select jobs to cancel',
									action: cancelVisibleJobs
								},
								{ displayName: 'Cancel all jobs matching filters', action: cancelFilteredJobs }
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<span class="text-xs min-w-[5rem]">Cancel jobs</span>
									<ChevronDown class="w-5 h-5" />
								</div>
							</svelte:fragment>
						</DropdownV2>
					{/if}
				</div>
			</div>
			<div class="relative flex gap-2 items-center pr-8 w-40">
				<Toggle
					size="xs"
					bind:checked={showSchedules}
					on:change={() => {
						localStorage.setItem('show_schedules_in_run', showSchedules ? 'true' : 'false')
					}}
				/>
				<span class="text-xs absolute -top-4"
					><span class="hidden xl:inline">CRON</span> Schedules</span
				>

				<Calendar size={16} />
			</div>
			<div class="relative flex gap-2 items-center pr-8 w-40">
				<span class="text-xs absolute -top-4">Planned later</span>
				<Toggle
					size="xs"
					bind:checked={showFutureJobs}
					on:change={() => {
						localStorage.setItem('show_future_jobs', showFutureJobs ? 'true' : 'false')
					}}
				/>
				<Clock size={16} />
			</div>
			<div class="flex flex-row gap-1 w-full max-w-lg">
				<div class="relative w-full">
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Min datetime</span>

						<input
							type="text"
							value={minTs
								? new Date(minTs).toLocaleString()
								: 'zoom x axis to set min (drag with ctrl)'}
							disabled
						/>

						<CalendarPicker
							clearable={true}
							date={minTs}
							label="Min datetimes"
							on:change={async ({ detail }) => {
								minTs = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
							on:clear={async () => {
								minTs = undefined
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
						/>
					</div>
				</div>
				<div class="relative w-full">
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Max</span>
						<input
							type="text"
							value={maxTs ? new Date(maxTs).toLocaleString() : 'zoom x axis to set max'}
							disabled
						/>
						<CalendarPicker
							clearable={true}
							date={maxTs}
							label="Max datetimes"
							on:change={async ({ detail }) => {
								maxTs = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
							on:clear={async () => {
								maxTs = undefined
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
						/>
					</div>
				</div>
			</div>
			<div class="flex flex-row gap-2 items-center">
				<Button size="xs" color="light" variant="border" on:click={reset}>Reset</Button>
				<ManuelDatePicker
					on:loadJobs={() => {
						lastFetchWentToEnd = false
						jobLoader?.loadJobs(minTs, maxTs, true, true)
					}}
					bind:minTs
					bind:maxTs
					bind:selectedManualDate
					{loading}
					bind:this={manualDatePicker}
				/>
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
							externalJobs={externalJobs ?? []}
							omittedObscuredJobs={extendedJobs?.omitted_obscured_jobs ?? false}
							showExternalJobs={!graphIsRunsChart}
							activeLabel={label}
							{isSelectingJobsToCancel}
							bind:selectedIds
							bind:selectedWorkspace
							bind:lastFetchWentToEnd
							on:loadExtra={loadExtra}
							on:filterByPath={filterByPath}
							on:filterByUser={filterByUser}
							on:filterByFolder={filterByFolder}
							on:filterByLabel={filterByLabel}
							on:filterByConcurrencyKey={filterByConcurrencyKey}
							on:filterByTag={filterByTag}
							on:filterBySchedule={filterBySchedule}
							bind:this={runsTable}
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
					{#if selectedIds.length === 1}
						{#if selectedIds[0] === '-'}
							<div class="p-4">There is no information available for this job</div>
						{:else}
							<JobPreview
								on:filterByConcurrencyKey={filterByConcurrencyKey}
								id={selectedIds[0]}
								workspace={selectedWorkspace}
							/>
						{/if}
					{:else if selectedIds.length > 1}
						<div class="text-xs m-4"
							>There are {selectedIds.length} jobs selected. Choose 1 to see detailed information</div
						>
					{:else}
						<div class="text-xs m-4">No job selected</div>
					{/if}
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>
	</div>
{:else}
	<div class="flex flex-col h-screen">
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
						All past and schedule executions of scripts and flows, including previews. You only see
						your own runs or runs of groups you belong to unless you are an admin.
					</Tooltip>
				</div>
				<RunsFilter
					bind:isSkipped
					{paths}
					{usernames}
					{folders}
					bind:jobKindsCat
					bind:folder
					bind:path
					bind:user
					bind:label
					bind:concurrencyKey
					bind:tag
					bind:success
					bind:argFilter
					bind:resultFilter
					bind:argError
					bind:resultError
					bind:allWorkspaces
					bind:schedulePath
					mobile={true}
					on:change={reloadJobsWithoutFilterError}
					on:sucessChange={(e) => {
						if (e.detail == 'running' && maxTs != undefined) {
							maxTs = undefined
						}
					}}
				/>
			</div>
		</div>
		<div class="p-2 w-full">
			<div class="relative z-10">
				<div class="absolute right-2">
					<ToggleButtonGroup
						bind:selected={graph}
						on:selected={() => {
							graphIsRunsChart = graph == 'RunChart'
						}}
					>
						<ToggleButton value="RunChart" label="Duration" />
						<ToggleButton value="ConcurrencyChart" label="Concurrency" />
					</ToggleButtonGroup>
					{#if !graphIsRunsChart}
						<DropdownV2
							items={[
								{
									displayName: 'None',
									action: () => setLookback(0)
								},
								{
									displayName: '1 day',
									action: () => setLookback(1)
								},
								{
									displayName: '3 days',
									action: () => setLookback(3)
								},
								{
									displayName: '7 days',
									action: () => setLookback(7)
								}
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<ChevronDown class="w-5 h-5" />
									<span class="text-xs min-w-[5rem]">{lookback} days lookback</span>
									<Tooltip>
										How far behind the min datetime to start considering jobs for the concurrency
										graph. Change this value to include jobs started before the set time window for
										the computation of the graph
									</Tooltip>
								</div>
							</svelte:fragment>
						</DropdownV2>
					{/if}
				</div>
			</div>
			{#if graph === 'RunChart'}
				<RunChart
					{lastFetchWentToEnd}
					bind:selectedIds
					canSelect={!isSelectingJobsToCancel}
					minTimeSet={minTs}
					maxTimeSet={maxTs}
					maxIsNow={maxTs == undefined}
					on:loadExtra={loadExtra}
					jobs={completedJobs}
					on:zoom={async (e) => {
						minTs = e.detail.min.toISOString()
						maxTs = e.detail.max.toISOString()
						manualDatePicker?.resetChoice()
						jobLoader?.loadJobs(minTs, maxTs, true)
					}}
					on:pointClicked={(e) => {
						runsTable.scrollToRun(e.detail)
					}}
				/>
			{:else if graph === 'ConcurrencyChart'}
				<ConcurrentJobsChart
					minTimeSet={minTs}
					maxTimeSet={maxTs}
					maxIsNow={maxTs == undefined}
					{extendedJobs}
					on:zoom={async (e) => {
						minTs = e.detail.min.toISOString()
						maxTs = e.detail.max.toISOString()
						jobLoader?.loadJobs(minTs, maxTs, true)
					}}
				/>
			{/if}
		</div>
		<div class="flex flex-col gap-4 md:flex-row w-full p-4 overflow-x-auto">
			<div class="flex items-center flex-row gap-2 grow">
				{#if queue_count}
					<RunsQueue
						{success}
						{queue_count}
						{suspended_count}
						on:jobs_waiting={() => {
							jobsFilter('waiting')
						}}
						on:jobs_suspended={() => {
							jobsFilter('suspended')
						}}
					/>
				{/if}
				<div class="flex flex-row">
					{#if isSelectingJobsToCancel}
						<div class="mt-1 p-2 h-8 flex flex-row items-center gap-1">
							<Button
								startIcon={{ icon: X }}
								size="xs"
								color="gray"
								variant="contained"
								on:click={() => {
									isSelectingJobsToCancel = false
									selectedIds = []
								}}
							/>
							<Button
								disabled={selectedIds.length == 0}
								startIcon={{ icon: Check }}
								size="xs"
								color="red"
								variant="contained"
								on:click={cancelSelectedJobs}
							>
								Cancel {jobCountString(selectedIds.length)}
							</Button>
						</div>
					{:else if !$userStore?.is_admin && !$superadmin}
						<DropdownV2
							items={[
								{
									displayName: 'Select jobs to cancel',
									action: cancelVisibleJobs
								}
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<span class="text-xs min-w-[5rem]">Cancel jobs</span>
									<ChevronDown class="w-5 h-5" />
								</div>
							</svelte:fragment>
						</DropdownV2>
					{:else}
						<DropdownV2
							items={[
								{
									displayName: 'Select jobs to cancel',
									action: cancelVisibleJobs
								},
								{ displayName: 'Cancel all jobs matching filters', action: cancelFilteredJobs }
							]}
						>
							<svelte:fragment slot="buttonReplacement">
								<div
									class="mt-1 p-2 h-8 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
								>
									<span class="text-xs min-w-[5rem]">Cancel jobs</span>
									<ChevronDown class="w-5 h-5" />
								</div>
							</svelte:fragment>
						</DropdownV2>
					{/if}
				</div>
			</div>
			<div class="flex gap-2 py-1">
				<div class="relative flex gap-2 items-center pr-8 w-20">
					<Toggle
						size="xs"
						bind:checked={showSchedules}
						on:change={() => {
							localStorage.setItem('show_schedules_in_run', showSchedules ? 'true' : 'false')
						}}
					/>
					<span class="text-xs absolute -top-4">Schedules</span>

					<Calendar size={16} />
				</div>
				<div class="relative flex gap-2 items-center pr-8 w-20">
					<span class="text-xs absolute -top-4">Planned later</span>
					<Toggle
						size="xs"
						bind:checked={showFutureJobs}
						on:change={() => {
							localStorage.setItem('show_future_jobs', showFutureJobs ? 'true' : 'false')
						}}
					/>
					<Clock size={16} />
				</div>
			</div>
			<div class="flex flex-row gap-1 w-full max-w-lg items-center">
				<div class="relative w-full">
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Min</span>

						<input
							type="text"
							class="min-w-10"
							value={minTs
								? new Date(minTs).toLocaleString()
								: 'zoom x axis to set min (drag with ctrl)'}
							disabled
						/>

						<CalendarPicker
							clearable={true}
							date={minTs}
							label="Min datetimes"
							on:change={async ({ detail }) => {
								minTs = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
							on:clear={async () => {
								minTs = undefined
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
						/>
					</div>
				</div>
				<div class="relative w-full">
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Max</span>
						<input
							class="min-w-10"
							type="text"
							value={maxTs ? new Date(maxTs).toLocaleString() : 'zoom x axis to set max'}
							disabled
						/>
						<CalendarPicker
							clearable={true}
							date={maxTs}
							label="Max datetimes"
							on:change={async ({ detail }) => {
								maxTs = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
							on:clear={async () => {
								maxTs = undefined
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobLoader?.loadJobs(minTs, maxTs, true)
								}, 1000)
							}}
						/>
					</div>
				</div>
			</div>
			<div class="flex flex-row gap-2 items-center">
				<Button size="xs" color="light" variant="border" on:click={reset}>Reset</Button>
				<ManuelDatePicker
					on:loadJobs={() => {
						lastFetchWentToEnd = false
						jobLoader?.loadJobs(minTs, maxTs, true, true)
					}}
					bind:this={manualDatePicker}
					bind:minTs
					bind:maxTs
					bind:selectedManualDate
					{loading}
				/>

				<Toggle
					size="xs"
					bind:checked={autoRefresh}
					options={{ right: 'Auto-refresh' }}
					textClass="whitespace-nowrap"
				/>
			</div>
		</div>
		<div class="grow">
			<RunsTable
				activeLabel={label}
				{jobs}
				externalJobs={externalJobs ?? []}
				omittedObscuredJobs={extendedJobs?.omitted_obscured_jobs ?? false}
				showExternalJobs={!graphIsRunsChart}
				{isSelectingJobsToCancel}
				bind:selectedIds
				bind:selectedWorkspace
				bind:lastFetchWentToEnd
				on:loadExtra={loadExtra}
				on:select={() => {
					if (!isSelectingJobsToCancel) runDrawer.openDrawer()
				}}
				on:filterByPath={filterByPath}
				on:filterByUser={filterByUser}
				on:filterByFolder={filterByFolder}
				on:filterByLabel={filterByLabel}
				on:filterByConcurrencyKey={filterByConcurrencyKey}
				on:filterByTag={filterByTag}
				bind:this={runsTable}
			/>
		</div>
	</div>
{/if}
