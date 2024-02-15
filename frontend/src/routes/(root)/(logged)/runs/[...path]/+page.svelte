<script lang="ts">
	import {
		JobService,
		Job,
		CompletedJob,
		UserService,
		FolderService,
		ScriptService,
		FlowService
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

	let jobs: Job[] | undefined
	let selectedId: string | undefined = undefined
	let selectedWorkspace: string | undefined = undefined

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

	let hideSchedules: boolean | undefined =
		$page.url.searchParams.get('hide_scheduled') != undefined
			? $page.url.searchParams.get('hide_scheduled') == 'true'
			: false

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

	let queue_count: Tweened<number> | undefined = undefined
	let jobKinds: string | undefined = undefined
	let loading: boolean = false
	let paths: string[] = []
	let usernames: string[] = []
	let folders: string[] = []
	let completedJobs: CompletedJob[] | undefined = undefined
	let argError = ''
	let resultError = ''
	let filterTimeout: NodeJS.Timeout | undefined = undefined
	let selectedManualDate = 0
	let autoRefresh: boolean = true
	let runDrawer: Drawer
	let cancelAllJobs = false
	let innerWidth = window.innerWidth
	let jobLoader: JobLoader | undefined = undefined

	let manualDatePicker: ManuelDatePicker

	$: (user ||
		folder ||
		path ||
		success !== undefined ||
		isSkipped ||
		hideSchedules ||
		argFilter ||
		resultFilter ||
		schedulePath ||
		jobKindsCat ||
		minTs ||
		maxTs ||
		allWorkspaces ||
		$workspaceStore) &&
		setQuery()

	function setQuery() {
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

		if (hideSchedules) {
			searchParams.set('hide_scheduled', hideSchedules.toString())
		} else {
			searchParams.delete('hide_scheduled')
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

		let newPath = path ? `/${path}` : '/'
		let newUrl = `/runs${newPath}?${searchParams.toString()}`
		history.replaceState(history.state, '', newUrl.toString())
	}

	function reloadLogsWithoutFilterError() {
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

		autoRefresh = true
		jobs = undefined
		completedJobs = undefined
		selectedManualDate = 0
		selectedId = undefined
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
</script>

<JobLoader
	{allWorkspaces}
	bind:jobs
	{user}
	{folder}
	{path}
	{success}
	{isSkipped}
	{argFilter}
	{resultFilter}
	{schedulePath}
	{jobKindsCat}
	computeMinAndMax={manualDatePicker?.computeMinMax}
	bind:minTs
	bind:maxTs
	{jobKinds}
	bind:queue_count
	{autoRefresh}
	bind:completedJobs
	{argError}
	{resultError}
	bind:loading
	bind:this={jobLoader}
/>

<ConfirmationModal
	title="Confirm cancelling all jobs"
	confirmationText="Cancel all jobs"
	open={cancelAllJobs}
	on:confirmed={async () => {
		cancelAllJobs = false
		let uuids = await JobService.cancelAll({ workspace: $workspaceStore ?? '' })
		jobLoader?.loadJobs(minTs, maxTs, true, true)
		sendUserToast(`Canceled ${uuids.length} jobs`)
	}}
	on:canceled={() => {
		cancelAllJobs = false
	}}
/>

<Drawer bind:this={runDrawer}>
	<DrawerContent title="Run details" on:close={runDrawer.closeDrawer}>
		{#if selectedId}
			<JobPreview blankLink id={selectedId} workspace={selectedWorkspace} />
		{/if}
	</DrawerContent>
</Drawer>

<svelte:window bind:innerWidth />

{#if innerWidth > 1280}
	<div class="w-full h-screen">
		<div class="px-2">
			<div class="flex items-center space-x-2 flex-row justify-between">
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
						All past and schedule executions of scripts and flows, including previews. You only see
						your own runs or runs of groups you belong to unless you are an admin.
					</Tooltip>
				</div>
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
					bind:hideSchedules
					bind:allWorkspaces
					on:change={reloadLogsWithoutFilterError}
					{usernames}
					{folders}
					{paths}
				/>
			</div>
		</div>

		<div class="p-2 w-full">
			<RunChart
				maxIsNow={maxTs == undefined}
				jobs={completedJobs}
				on:zoom={async (e) => {
					minTs = e.detail.min.toISOString()
					maxTs = e.detail.max.toISOString()
					jobLoader?.loadJobs(minTs, maxTs, true)
				}}
			/>
		</div>
		<div class="flex flex-col gap-1 md:flex-row w-full p-4">
			<div class="flex gap-2 grow flex-row">
				<RunsQueue {queue_count} {allWorkspaces} />
				<Button
					size="xs"
					color="light"
					variant="contained"
					title="Require to be an admin. Cancel all jobs in queue"
					disabled={!$userStore?.is_admin && !$superadmin}
					on:click={async () => (cancelAllJobs = true)}>Cancel All</Button
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
				<Button size="xs" color="light" variant="border" on:click={reset}>Reset</Button>
				<ManuelDatePicker
					on:loadJobs={() => {
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
							bind:selectedId
							bind:selectedWorkspace
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
						<JobPreview id={selectedId} workspace={selectedWorkspace} />
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
					bind:success
					bind:argFilter
					bind:resultFilter
					bind:argError
					bind:resultError
					bind:hideSchedules
					bind:allWorkspaces
					mobile={true}
					on:change={reloadLogsWithoutFilterError}
				/>
			</div>
		</div>
		<div class="p-2 w-full">
			<RunChart
				maxIsNow={maxTs == undefined}
				jobs={completedJobs}
				on:zoom={async (e) => {
					minTs = e.detail.min.toISOString()
					maxTs = e.detail.max.toISOString()
					jobLoader?.loadJobs(minTs, maxTs, true)
				}}
			/>
		</div>
		<div class="flex flex-col gap-1 md:flex-row w-full p-4">
			<div class="flex items-center flex-row gap-2 grow mb-4">
				{#if queue_count}
					<RunsQueue {queue_count} {allWorkspaces} />
				{/if}
				<Button
					size="xs"
					color="light"
					variant="contained"
					title="Require to be an admin. Cancel all jobs in queue"
					disabled={!$userStore?.is_admin && !$superadmin}
					on:click={async () => (cancelAllJobs = true)}>Cancel All</Button
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
				<Button size="xs" color="light" variant="border" on:click={reset}>Reset</Button>
				<ManuelDatePicker
					on:loadJobs={() => {
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
				{jobs}
				bind:selectedId
				bind:selectedWorkspace
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
	</div>
{/if}
