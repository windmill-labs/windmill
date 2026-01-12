<script lang="ts">
	import {
		JobService,
		type Job,
		type CompletedJob,
		UserService,
		FolderService,
		ScriptService,
		FlowService,
		type ExtendedJobs,
		OpenAPI,
		type JobTriggerKind
	} from '$lib/gen'

	import { sendUserToast } from '$lib/toast'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import RunChart from '$lib/components/RunChart.svelte'

	import JobRunsPreview from '$lib/components/runs/JobRunsPreview.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import CalendarPicker from '$lib/components/common/calendarPicker/CalendarPicker.svelte'

	import RunsTable from '$lib/components/runs/RunsTable.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RunsFilter from '$lib/components/runs/RunsFilter.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import type { Tweened } from 'svelte/motion'
	import RunsQueue from '$lib/components/runs/RunsQueue.svelte'
	import { twMerge } from 'tailwind-merge'
	import ManuelDatePicker from '$lib/components/runs/ManuelDatePicker.svelte'
	import JobsLoader from '$lib/components/runs/JobsLoader.svelte'
	import { Calendar, Clock, TriangleAlert } from 'lucide-svelte'
	import ConcurrentJobsChart from '$lib/components/ConcurrentJobsChart.svelte'
	import { isJobSelectable, type RunsSelectionMode } from '$lib/utils'
	import BatchReRunOptionsPane, {
		type BatchReRunOptions
	} from '$lib/components/runs/BatchReRunOptionsPane.svelte'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import RunOption from '$lib/components/runs/RunOption.svelte'
	import TooltipV2 from '$lib/components/meltComponents/Tooltip.svelte'
	import DropdownSelect from '$lib/components/DropdownSelect.svelte'
	import RunsBatchActionsDropdown from '$lib/components/runs/RunsBatchActionsDropdown.svelte'
	import { createBubbler } from 'svelte/legacy'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import AnimatedPane from '$lib/components/splitPanes/AnimatedPane.svelte'
	import z from 'zod'
	import { useSearchParams } from '$lib/svelte5Utils.svelte'

	let filters = useSearchParams(
		z.object({
			path: z.string().nullable().default(null),
			worker: z.string().nullable().default(null),
			user: z.string().nullable().default(null),
			folder: z.string().nullable().default(null),
			label: z.string().nullable().default(null),
			concurrency_key: z.string().nullable().default(null),
			tag: z.string().nullable().default(null),
			allow_wildcards: z.boolean().default(false),
			show_future_jobs: z.boolean().default(true),
			success: z
				.enum(['running', 'suspended', 'waiting', 'success', 'failure'])
				.nullable()
				.default(null),
			show_skipped: z.boolean().default(false),
			show_schedules: z.boolean().default(true),
			min_ts: z.string().nullable().default(null),
			max_ts: z.string().nullable().default(null),
			schedule_path: z.string().nullable().default(null),
			job_kinds: z.string().default('runs'),
			all_workspaces: z.boolean().default(false),
			arg: z
				.string()
				.transform((s) => JSON.parse(s ?? '{}'))
				.nullable()
				.default(null),
			result: z
				.string()
				.transform((s) => JSON.parse(s ?? '{}'))
				.nullable()
				.default(null),
			job_trigger_kind: z
				.string()
				.transform((s) => s as JobTriggerKind)
				.nullable()
				.default(null),
			per_page: z.number().default(1000)
		})
	)
	let jobs: Job[] | undefined = $state()
	let selectedIds: string[] = $state([])
	let loadingSelectedIds = $state(false)
	let selectedWorkspace: string | undefined = $state(undefined)

	let batchReRunOptions: BatchReRunOptions = $state({ flow: {}, script: {} })

	let lastFetchWentToEnd = $state(false)

	let queue_count: Tweened<number> | undefined = $state(undefined)
	let suspended_count: Tweened<number> | undefined = $state(undefined)

	let jobKinds: string | undefined = $state(undefined)
	let loading: boolean = $state(false)
	let paths: string[] = $state([])
	let usernames: string[] = $state([])
	let folders: string[] = $state([])
	let completedJobs: CompletedJob[] | undefined = $state(undefined)
	let extendedJobs: ExtendedJobs | undefined = $state(undefined)
	let argError = $state('')
	let resultError = $state('')
	let filterTimeout: ReturnType<typeof setInterval> | undefined = undefined
	let selectedManualDate = $state(0)
	let autoRefresh: boolean = $state(getAutoRefresh())
	let runDrawer: Drawer | undefined = $state(undefined)
	let lookback: number = $state(1)
	let askingForConfirmation:
		| undefined
		| {
				title: string
				confirmBtnText: string
				loading?: boolean
				preContent?: string
				onConfirm?: (forceCancel: boolean) => void
				type?: ConfirmationModal['$$prop_def']['type']
		  } = $state(undefined)

	function getAutoRefresh() {
		try {
			return localStorage.getItem('auto_refresh_in_runs') != 'false'
		} catch (e) {
			console.error('Error getting auto refresh', e)
			return true
		}
	}

	let innerWidth = $state(window.innerWidth)
	let jobsLoader: JobsLoader | undefined = $state(undefined)
	let externalJobs: Job[] | undefined = $state(undefined)

	let graph: 'RunChart' | 'ConcurrencyChart' = $state(
		typeOfChart(page.url.searchParams.get('graph'))
	)
	let graphIsRunsChart: boolean = $state(untrack(() => graph) === 'RunChart')

	let manualDatePicker: ManuelDatePicker | undefined = $state(undefined)

	let runsTable: RunsTable | undefined = $state(undefined)

	function reloadJobsWithoutFilterError() {
		if (resultError == '' && argError == '') {
			filterTimeout && clearTimeout(filterTimeout)
			filterTimeout = setTimeout(() => {
				jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
			}, 2000)
		}
	}

	function reset() {
		filters.min_ts = null
		filters.max_ts = null
		jobs = undefined
		completedJobs = undefined
		lastFetchWentToEnd = false
		selectedManualDate = 0
		selectedIds = []
		filters.schedule_path = null
		batchReRunOptions = { flow: {}, script: {} }
		selectionMode = false
		selectedWorkspace = undefined
		jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
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

	function filterByPath(e: CustomEvent<string>) {
		filters.path = e.detail
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = null
		filters.schedule_path = null
	}

	function filterByUser(e: CustomEvent<string>) {
		filters.path = null
		filters.folder = null
		filters.user = e.detail
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.schedule_path = null
	}

	function filterByFolder(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = e.detail
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = null
		filters.schedule_path = null
	}

	function filterByLabel(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = e.detail
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = null
		filters.allow_wildcards = false
		filters.schedule_path = null
	}

	function filterByConcurrencyKey(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = e.detail
		filters.tag = null
		filters.worker = null
		filters.schedule_path = null
	}

	function filterByTag(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = null
		filters.tag = e.detail
		filters.worker = null
		filters.allow_wildcards = false
		filters.schedule_path = null
	}

	function filterBySchedule(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = null
		filters.schedule_path = e.detail
	}

	function filterByWorker(e: CustomEvent<string>) {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = e.detail
		filters.allow_wildcards = false
		filters.schedule_path = null
	}

	let calendarChangeTimeout: ReturnType<typeof setInterval> | undefined = $state(undefined)

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

	let selectionMode: RunsSelectionMode | false = $state(false)

	async function onSetSelectionMode(mode: RunsSelectionMode | false) {
		selectionMode = mode
		if (!mode) {
			selectedIds = []
			batchReRunOptions = { flow: {}, script: {} }
			return
		}
		const selectableIds = jobs?.filter(isJobSelectable(mode)).map((j) => j.id) ?? []
		selectedIds = []

		if (!selectableIds?.length) {
			sendUserToast(
				'There are no visible jobs that can be ' +
					{ cancel: 'cancelled', 're-run': 're-ran' }[mode],
				true
			)
		}
	}

	function getSelectedFilters() {
		return {
			workspace: $workspaceStore ?? '',
			startedBefore: filters.max_ts ?? undefined,
			startedAfter: filters.min_ts ?? undefined,
			schedulePath: filters.schedule_path ?? undefined,
			scriptPathExact: filters.path === null || filters.path === '' ? undefined : filters.path,
			createdBy: filters.user || undefined,
			scriptPathStart: filters.folder ? `f/${filters.folder}/` : undefined,
			jobKinds: jobKinds == '' ? undefined : jobKinds,
			success:
				filters.success == 'success' ? true : filters.success == 'failure' ? false : undefined,
			running:
				filters.success == 'running' || filters.success == 'suspended'
					? true
					: filters.success == 'waiting'
						? false
						: undefined,
			isSkipped: filters.show_skipped ? undefined : false,
			// isFlowStep: jobKindsCat != 'all' ? false : undefined,
			hasNullParent:
				filters.path != undefined || filters.path != undefined || filters.job_kinds != 'all'
					? true
					: undefined,
			label: filters.label || undefined,
			tag: filters.tag || undefined,
			isNotSchedule: filters.show_schedules == false ? true : undefined,
			suspended:
				filters.success == 'waiting' ? false : filters.success == 'suspended' ? true : undefined,
			scheduledForBeforeNow:
				filters.show_future_jobs == false ||
				filters.success == 'waiting' ||
				filters.success == 'suspended'
					? true
					: undefined,
			args: filters.arg && Object.keys(filters.arg).length && !argError ? filters.arg : undefined,
			result:
				filters.result && Object.keys(filters.result).length && !resultError
					? filters.result
					: undefined,
			jobTriggerKind: filters.job_trigger_kind || undefined,
			allWorkspaces: filters.all_workspaces || undefined,
			allowWildcards: filters.allow_wildcards || undefined
		}
	}

	$effect(() => {
		if (filters.job_trigger_kind === 'schedule' && !filters.show_schedules) {
			filters.show_schedules = true
		}
	})

	async function cancelJobs(uuidsToCancel: string[], forceCancel: boolean = false) {
		const uuids = await JobService.cancelSelection({
			workspace: $workspaceStore ?? '',
			requestBody: uuidsToCancel,
			forceCancel: forceCancel
		})
		selectedIds = []
		jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true, true)
		sendUserToast(`Canceled ${uuids.length} jobs`)
		selectionMode = false
	}

	async function onCancelFilteredJobs() {
		forceCancelInPopup = false
		askingForConfirmation = {
			title: 'Confirm cancelling all jobs corresponding to the selected filters',
			confirmBtnText: 'Loading...',
			loading: true
		}

		const selectedFilters = getSelectedFilters()
		const selectedFiltersString = JSON.stringify(selectedFilters, null, 4)
		const jobIdsToCancel = await JobService.listFilteredQueueUuids(selectedFilters)

		askingForConfirmation = {
			title: `Confirm cancelling all jobs corresponding to the selected filters (${jobIdsToCancel.length} jobs)`,
			confirmBtnText: `Cancel ${jobIdsToCancel.length} jobs that matched the filters`,
			preContent: selectedFiltersString,
			onConfirm: (forceCancel) => {
				cancelJobs(jobIdsToCancel, forceCancel)
			}
		}
	}

	async function onCancelSelectedJobs() {
		forceCancelInPopup = true
		askingForConfirmation = {
			confirmBtnText: `Cancel ${selectedIds.length} jobs`,
			title: 'Confirm cancelling the selected jobs',
			onConfirm: (forceCancel) => {
				cancelJobs(selectedIds, forceCancel)
			}
		}
	}

	async function reRunJobs(jobIdsToReRun: string[]) {
		if (!$workspaceStore) return

		if (askingForConfirmation) {
			askingForConfirmation.loading = true
		}

		const body: Parameters<typeof JobService.batchReRunJobs>[0]['requestBody'] = {
			job_ids: jobIdsToReRun,
			script_options_by_path: batchReRunOptions.script,
			flow_options_by_path: batchReRunOptions.flow
		}

		// workaround because EventSource does not support POST requests
		// https://medium.com/@david.richards.tech/sse-server-sent-events-using-a-post-request-without-eventsource-1c0bd6f14425
		const response = await fetch(`${OpenAPI.BASE}/w/${$workspaceStore}/jobs/run/batch_rerun_jobs`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(body)
		})
		await new Promise(async (resolve) => {
			const reader = response?.body?.pipeThrough(new TextDecoderStream()).getReader()
			let reRanUuids: string[] = []
			if (reader) {
				while (true) {
					const { value, done } = await reader.read()
					if (value) {
						// It is possible get multiple values at once in case of buffering
						const receivedUuids: string[] = []
						for (const line of value.split('\n')) {
							if (!line) continue
							else if (line.startsWith('Error:')) {
								console.error(line)
							} else {
								receivedUuids.push(line)
							}
						}
						if (receivedUuids.length) {
							reRanUuids.push(...receivedUuids)
							if (askingForConfirmation) {
								askingForConfirmation.confirmBtnText = `${reRanUuids.length}/${jobIdsToReRun.length}`
							}
						}
					}

					if (done || !value) {
						if (reRanUuids.length) {
							sendUserToast(`Re-ran ${reRanUuids.length}/${jobIdsToReRun.length} jobs`)
						}
						if (reRanUuids.length !== jobIdsToReRun.length) {
							sendUserToast(
								`Failed to re-run ${jobIdsToReRun.length - reRanUuids.length} jobs. Check console for details`,
								true
							)
							// We do not get explicit error from backend if the job script don't exist
							for (const jobId of jobIdsToReRun) {
								if (reRanUuids.includes(jobId)) continue
								console.error('Could not re-run job ' + jobId)
							}
						}
						break
					}
				}
			}
			resolve(undefined)
		})

		selectedIds = []
		batchReRunOptions = { flow: {}, script: {} }
		jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true, true)
		selectionMode = false
	}

	async function onReRunFilteredJobs() {
		const selectedFilters = getSelectedFilters()
		selectedIds = []
		loadingSelectedIds = true

		if (filters.job_kinds !== 'runs') {
			sendUserToast('Batch re-run is only supported for scripts and flows', true)
		}
		selectedIds = await JobService.listFilteredJobsUuids({
			...selectedFilters,
			jobKinds: 'script,flow'
		})
		selectionMode = 're-run'
	}

	async function onReRunSelectedJobs() {
		const jobIdsToReRun = selectedIds
		askingForConfirmation = {
			title: `Confirm re-running the selected jobs`,
			confirmBtnText: `Re-run ${jobIdsToReRun.length} jobs`,
			type: 'reload',
			onConfirm: async () => {
				await reRunJobs(jobIdsToReRun)
			}
		}
	}

	function setLookback(lookbackInDays: number) {
		lookback = lookbackInDays
	}

	async function loadExtra() {
		if (jobsLoader) {
			lastFetchWentToEnd = await jobsLoader.loadExtraJobs()
		}
	}

	function jobsFilter(f: 'waiting' | 'suspended') {
		filters.path = null
		filters.user = null
		filters.folder = null
		filters.label = null
		filters.concurrency_key = null
		filters.tag = null
		filters.worker = null
		filters.schedule_path = null
		if (filters.success == f) {
			filters.success = null
		} else {
			filters.success = f
		}
		filters.job_kinds = 'all'
	}

	$effect(() => {
		loadingSelectedIds && selectedIds.length && setTimeout(() => (loadingSelectedIds = false), 250)
	})
	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadUsernames()
				loadFolders()
				loadPaths()
			})
		}
	})
	let warnJobLimit = $derived.by(() => {
		let extended = extendedJobs as ExtendedJobs | undefined
		return (
			graph === 'ConcurrencyChart' &&
			extended !== undefined &&
			extended.jobs.length + extended.obscured_jobs.length >= 1000
		)
	})

	const bubble = createBubbler()

	function selectAll() {
		if (!selectionMode) return
		if (allSelected) {
			allSelected = false
			selectedIds = []
		} else {
			allSelected = true
			selectedIds = jobs?.filter(isJobSelectable(selectionMode)).map((j) => j.id) ?? []
		}
	}

	let allSelected = $derived.by(() => {
		return selectionMode && selectedIds.length === selectableJobCount
	})

	const selectableJobCount = $derived.by(() => {
		if (!selectionMode) return 0
		return jobs?.filter(isJobSelectable(selectionMode)).length ?? 0
	})

	let tableTopBarWidth = $state(0)

	const smallScreenWidth = 1920
	const verySmallScreenWidth = 1300

	let forceCancelInPopup = $state(false)

	const warnJobLimitMsg = $derived(
		`The exact number of concurrent jobs at the beginning of the time range may be incorrect as only the last ${filters.per_page} jobs are taken into account: a job that was started earlier than this limit will not be taken into account`
	)
</script>

<JobsLoader
	allowWildcards={filters.allow_wildcards}
	allWorkspaces={filters.all_workspaces}
	bind:jobs
	user={filters.user}
	folder={filters.folder}
	worker={filters.worker}
	label={filters.label}
	concurrencyKey={filters.concurrency_key}
	tag={filters.tag}
	path={filters.path}
	success={filters.success}
	showSkipped={filters.show_skipped}
	argFilter={filters.arg}
	resultFilter={filters.result}
	jobTriggerKind={filters.job_trigger_kind}
	showSchedules={filters.show_schedules}
	showFutureJobs={filters.show_future_jobs}
	schedulePath={filters.schedule_path}
	jobKindsCat={filters.job_kinds}
	computeMinAndMax={manualDatePicker?.computeMinMax}
	bind:minTs={filters.min_ts}
	bind:maxTs={filters.max_ts}
	bind:jobKinds
	bind:queue_count
	bind:suspended_count
	{autoRefresh}
	bind:completedJobs
	bind:externalJobs
	bind:extendedJobs
	{argError}
	{resultError}
	bind:perPage={filters.per_page}
	bind:loading
	bind:this={jobsLoader}
	lookback={graphIsRunsChart ? 0 : lookback}
/>

<ConfirmationModal
	title={askingForConfirmation?.title ?? ''}
	confirmationText={askingForConfirmation?.confirmBtnText ?? ''}
	open={!!askingForConfirmation}
	on:confirmed={async () => {
		const func = askingForConfirmation?.onConfirm
		await func?.(forceCancelInPopup)
		askingForConfirmation = undefined
	}}
	type={askingForConfirmation?.type}
	loading={askingForConfirmation?.loading}
	on:canceled={() => {
		askingForConfirmation = undefined
	}}
>
	{#if askingForConfirmation?.preContent}
		<pre>{askingForConfirmation.preContent}</pre>
		<Toggle
			size="xs"
			class="mt-4"
			color="red"
			bind:checked={forceCancelInPopup}
			options={{
				right: 'Force cancel',
				rightTooltip:
					'Only use this for jobs that refuse to gracefully cancel. This is dangerous, only do this if you have no alternatives!'
			}}
		></Toggle>
		{#if forceCancelInPopup}
			<div class="mt-4 text-red-500 p-2 text-sm">
				<p>
					Force cancel is enabled. This is dangerous, only do this if you have no alternatives.
					Instead of being gracefully cancelled, all jobs will be immediately sent to the completed
					job table regardless of them being processed or not or part of running flows. You may end
					up in an inconsistent state.
				</p>
			</div>
		{/if}
	{/if}
</ConfirmationModal>

<Drawer bind:this={runDrawer}>
	<DrawerContent title="Run details" on:close={runDrawer.closeDrawer}>
		{#if selectedIds.length === 1}
			{#if selectedIds[0] === '-'}
				<div class="p-4">There is no information available for this job</div>
			{:else}
				<JobRunsPreview blankLink id={selectedIds[0]} workspace={selectedWorkspace} />
			{/if}
		{/if}
	</DrawerContent>
</Drawer>

<svelte:window
	onpopstate={() => {
		reset()
	}}
/>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.runs}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<div class="w-full h-screen flex flex-col" bind:clientWidth={innerWidth}>
		<!-- Header and filters -->
		<div class="flex flex-row items-start w-full border-b px-4 gap-8">
			<div class="flex flex-row items-center h-full gap-6">
				<div class="flex flex-row items-center gap-1">
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

				<!-- Queue -->
				<RunsQueue
					success={filters.success}
					{queue_count}
					{suspended_count}
					onJobsWaiting={() => {
						jobsFilter('waiting')
					}}
					onJobsSuspended={() => {
						jobsFilter('suspended')
					}}
					small={innerWidth < smallScreenWidth}
				/>
			</div>

			<div class="py-2 flex items-start gap-x-4 gap-y-2 flex-row grow min-w-0 justify-end">
				<!-- Dates -->
				<div class="flex flex-row gap-2">
					<RunOption label="From" for="min-datetimes">
						{#if filters.min_ts || filters.max_ts}
							<input
								type="text"
								class="!text-sm text-primary !bg-surface-secondary h-9 !border-none"
								value={filters.min_ts
									? new Date(filters.min_ts).toLocaleString()
									: 'zoom x axis to set min'}
								disabled
								name="min-datetimes"
							/>
						{/if}
						<CalendarPicker
							clearable={true}
							date={filters.min_ts}
							label="From"
							class={filters.min_ts || filters.max_ts
								? ''
								: 'relative top-0 bottom-0 left-0 right-0 h-[34px]'}
							on:change={async ({ detail }) => {
								filters.min_ts = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
								}, 1000)
							}}
							on:clear={async () => {
								filters.min_ts = null
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
								}, 1000)
							}}
						/>
					</RunOption>

					<RunOption label="To" for="max-datetimes">
						{#if filters.max_ts || filters.min_ts}
							<input
								type="text"
								class="!text-sm text-primary !bg-surface-secondary h-9 !border-none"
								value={filters.max_ts
									? new Date(filters.max_ts).toLocaleString()
									: 'zoom x axis to set max'}
								name="max-datetimes"
								disabled
							/>
						{/if}
						<CalendarPicker
							clearable={true}
							date={filters.max_ts}
							label="To"
							class={filters.min_ts || filters.max_ts
								? ''
								: 'relative top-0 bottom-0 left-0 right-0 h-[34px]'}
							on:change={async ({ detail }) => {
								filters.max_ts = new Date(detail).toISOString()
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
								}, 1000)
							}}
							on:clear={async () => {
								filters.max_ts = null
								calendarChangeTimeout && clearTimeout(calendarChangeTimeout)
								calendarChangeTimeout = setTimeout(() => {
									jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
								}, 1000)
							}}
						/>
					</RunOption>

					{#if filters.min_ts || filters.max_ts}
						<RunOption label="Reset" for="reset" noLabel>
							<Button variant="default" size="xs" onClick={reset} btnClasses="h-9">Reset</Button>
						</RunOption>
					{/if}
				</div>

				<!-- Filters 1 -->
				<div class="flex flex-row gap-2">
					<RunsFilter
						bind:allowWildcards={filters.allow_wildcards}
						bind:user={filters.user}
						bind:folder={filters.folder}
						bind:label={filters.label}
						bind:concurrencyKey={filters.concurrency_key}
						bind:tag={filters.tag}
						bind:worker={filters.worker}
						bind:showSkipped={filters.show_skipped}
						bind:path={filters.path}
						bind:success={filters.success}
						bind:argFilter={filters.arg}
						bind:resultFilter={filters.result}
						bind:jobTriggerKind={filters.job_trigger_kind}
						bind:argError
						bind:resultError
						bind:jobKindsCat={filters.job_kinds}
						bind:allWorkspaces={filters.all_workspaces}
						bind:schedulePath={filters.schedule_path}
						on:change={reloadJobsWithoutFilterError}
						on:successChange={(e) => {
							if (e.detail == 'running' && filters.max_ts != undefined) {
								filters.max_ts = null
							}
						}}
						{usernames}
						{folders}
						{paths}
						mobile={innerWidth < verySmallScreenWidth}
						small={innerWidth < smallScreenWidth}
						calendarSmall={!filters.min_ts && !filters.max_ts}
					/>
				</div>
			</div>
		</div>

		<!-- Graph -->
		<div class="p-2 px-4 pt-8 w-full border-b">
			<div class="relative z-10">
				<div class="absolute left-0 -top-7 flex flex-row gap-2 items-center min-w-24">
					<ToggleButtonGroup
						selected={graph}
						on:selected={({ detail }) => {
							graph = detail
							graphIsRunsChart = graph === 'RunChart'
						}}
					>
						{#snippet children({ item })}
							<ToggleButton value="RunChart" label="Duration" {item} />
							<ToggleButton
								{item}
								value="ConcurrencyChart"
								label="Concurrency"
								icon={warnJobLimit ? TriangleAlert : undefined}
								tooltip={warnJobLimit ? warnJobLimitMsg : undefined}
							/>
						{/snippet}
					</ToggleButtonGroup>

					{#if !graphIsRunsChart}
						<DropdownSelect
							items={[
								{
									displayName: 'None',
									action: () => setLookback(0),
									id: '0'
								},
								{
									displayName: '1 day',
									action: () => setLookback(1),
									id: '1'
								},
								{
									displayName: '3 days',
									action: () => setLookback(3),
									id: '3'
								},
								{
									displayName: '7 days',
									action: () => setLookback(7),
									id: '7'
								}
							]}
							selected={lookback.toString()}
							selectedDisplayName={`${lookback} days lookback`}
						>
							{#snippet extraLabel()}
								<TooltipV2>
									{#snippet text()}
										How far behind the min datetime to start considering jobs for the concurrency
										graph. Change this value to include jobs started before the set time window for
										the computation of the graph
									{/snippet}
								</TooltipV2>
							{/snippet}
						</DropdownSelect>
					{/if}
				</div>
			</div>
			{#if graph === 'RunChart'}
				<RunChart
					{lastFetchWentToEnd}
					bind:selectedIds
					canSelect={!selectionMode}
					minTimeSet={filters.min_ts}
					maxTimeSet={filters.max_ts}
					totalRowsFetched={jobs?.length ?? 0}
					maxIsNow={filters.max_ts == undefined}
					onLoadExtra={loadExtra}
					jobs={completedJobs}
					onZoom={async (zoom) => {
						filters.min_ts = zoom.min.toISOString()
						filters.max_ts = zoom.max.toISOString()
						manualDatePicker?.resetChoice()
						jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
					}}
					onPointClicked={(ids) => {
						runsTable?.scrollToRun(ids)
					}}
				/>
			{:else if graph === 'ConcurrencyChart'}
				<ConcurrentJobsChart
					minTimeSet={filters.min_ts}
					maxTimeSet={filters.max_ts}
					maxIsNow={filters.max_ts == undefined}
					{extendedJobs}
					onZoom={async (zoom) => {
						filters.min_ts = zoom.min.toISOString()
						filters.max_ts = zoom.max.toISOString()
						jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
					}}
				/>
			{/if}
		</div>

		<div class="grow min-h-0">
			<Splitpanes>
				<Pane minSize={40}>
					<div class="flex flex-col h-full">
						<!-- Runs table top bar -->
						<div
							class="flex flex-row gap-4 items-center px-2 py-1 grow-0 justify-between"
							bind:clientWidth={tableTopBarWidth}
						>
							<div class="flex flex-row gap-4 items-center">
								{#if selectionMode && selectableJobCount}
									<div class="flex flex-row items-center font-semibold text-sm">
										<div class="px-2">
											<input
												onfocus={bubble('focus')}
												type="checkbox"
												checked={allSelected}
												id="select-all"
												class={twMerge(
													'cursor-pointer',
													allSelected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
													'flex flex-row items-center p-2 pr-4 top-0 font-semibold text-sm'
												)}
												onclick={selectAll}
											/>
										</div>
										<label
											class="cursor-pointer whitespace-nowrap text-xs text-emphasis font-semibold"
											for="select-all">Select all</label
										>
									</div>
								{/if}

								<RunsBatchActionsDropdown
									isLoading={loadingSelectedIds}
									{selectionMode}
									selectionCount={selectedIds.length}
									{onSetSelectionMode}
									{onCancelFilteredJobs}
									{onCancelSelectedJobs}
									{onReRunFilteredJobs}
									{onReRunSelectedJobs}
									small={tableTopBarWidth < 800}
								/>
							</div>

							<div class="flex flex-row gap-4 items-center">
								{#if !filters.job_trigger_kind}
									<div class="flex flex-row gap-1 items-center">
										<Toggle
											id="cron-schedules"
											bind:checked={filters.show_schedules}
											options={tableTopBarWidth < 800 || selectionMode
												? {}
												: { right: 'Schedules' }}
										/>
										<span title="Schedules">
											<Calendar size="14" />
										</span>
									</div>
								{/if}

								<div class="flex flex-row gap-1 items-center">
									<Toggle
										size="sm"
										bind:checked={filters.show_future_jobs}
										id="planned-later"
										options={tableTopBarWidth < 800 || selectionMode
											? {}
											: { right: 'Planned later' }}
									/>
									<span title="Planned later">
										<Clock size={14} />
									</span>
								</div>
								<div class="flex flex-row gap-2 items-center">
									<ManuelDatePicker
										on:loadJobs={() => {
											lastFetchWentToEnd = false
											jobsLoader?.loadJobs(filters.min_ts, filters.max_ts, true)
										}}
										bind:minTs={filters.min_ts}
										bind:maxTs={filters.max_ts}
										bind:selectedManualDate
										{loading}
										bind:this={manualDatePicker}
										numberOfLastJobsToFetch={filters.per_page}
									/>
									<Toggle
										size="sm"
										bind:checked={autoRefresh}
										on:change={() => {
											localStorage.setItem('auto_refresh_in_runs', autoRefresh ? 'true' : 'false')
										}}
										options={{ right: 'Auto-refresh' }}
										textClass="whitespace-nowrap"
									/>
								</div>
							</div>
						</div>

						<!-- Runs table. Add overflow-hidden because scroll is handled inside the runs table based on this wrapper height -->
						<div class="grow min-h-0 overflow-y-hidden overflow-x-auto">
							{#if jobs}
								<RunsTable
									{jobs}
									externalJobs={externalJobs ?? []}
									omittedObscuredJobs={extendedJobs?.omitted_obscured_jobs ?? false}
									showExternalJobs={!graphIsRunsChart}
									activeLabel={filters.label}
									{selectionMode}
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
									on:filterByWorker={filterByWorker}
									bind:this={runsTable}
									perPage={filters.per_page}
								></RunsTable>
							{:else}
								<div class="gap-1 flex flex-col">
									{#each new Array(8) as _}
										<Skeleton layout={[[3]]} />
									{/each}
								</div>
							{/if}
						</div>
						<div
							class="bg-surface-secondary border-t flex text-xs px-2 py-1 items-center justify-end gap-2"
						>
							Per page:
							<Select
								class="w-20"
								bind:value={
									() => filters.per_page,
									(newPerPage) => {
										filters.per_page = newPerPage
										if (newPerPage > (jobs?.length ?? 1000)) loadExtra()
									}
								}
								onCreateItem={(v) => (filters.per_page = parseInt(v))}
								items={[
									{ value: 25, label: '25' },
									{ value: 100, label: '100' },
									{ value: 1000, label: '1000' },
									{ value: 10000, label: '10000' }
								]}
							/>
						</div>
					</div>
				</Pane>
				<AnimatedPane size={40} minSize={15} class="flex flex-col" opened={selectedIds.length > 0}>
					{#if selectionMode === 're-run'}
						<BatchReRunOptionsPane {selectedIds} bind:options={batchReRunOptions} />
					{:else if selectedIds.length === 1}
						{#if selectedIds[0] === '-'}
							<div class="p-4">There is no information available for this job</div>
						{:else}
							<JobRunsPreview
								on:filterByConcurrencyKey={filterByConcurrencyKey}
								on:filterByWorker={filterByWorker}
								id={selectedIds[0]}
								workspace={selectedWorkspace}
							/>
						{/if}
					{:else if selectedIds.length > 1}
						<div class="text-xs m-4"
							>There are {selectedIds.length} jobs selected. Choose 1 to see detailed information</div
						>
					{/if}
				</AnimatedPane>
			</Splitpanes>
		</div>
	</div>
{/if}
