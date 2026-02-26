<script lang="ts">
	import {
		JobService,
		UserService,
		FolderService,
		ScriptService,
		FlowService,
		type ExtendedJobs,
		OpenAPI
	} from '$lib/gen'

	import { sendUserToast } from '$lib/toast'
	import { userStore, workspaceStore, userWorkspaces, superadmin } from '$lib/stores'
	import {
		Button,
		ButtonType,
		Drawer,
		DrawerContent,
		Skeleton,
		Tab,
		Tabs
	} from '$lib/components/common'
	import RunChart from '$lib/components/RunChart.svelte'

	import JobRunsPreview from '$lib/components/runs/JobRunsPreview.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import RunsTable from '$lib/components/runs/RunsTable.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import {
		allowWildcards,
		buildRunsFilterSearchbarSchema,
		buildRunsFilterPresets
	} from '$lib/components/runs/runsFilter'
	import Toggle from '$lib/components/Toggle.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import RunsQueue from '$lib/components/runs/RunsQueue.svelte'
	import { twMerge } from 'tailwind-merge'
	import { computeJobKinds, useJobsLoader } from '$lib/components/runs/useJobsLoader.svelte'
	import ConcurrentJobsChart from '$lib/components/ConcurrentJobsChart.svelte'
	import { pluralize } from '$lib/utils'
	import BatchReRunOptionsPane, {
		type BatchReRunOptions
	} from '$lib/components/runs/BatchReRunOptionsPane.svelte'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import Select from '$lib/components/select/Select.svelte'
	import AnimatedPane from '$lib/components/splitPanes/AnimatedPane.svelte'
	import { StaleWhileLoading, useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import {
		Calendar,
		CircleAlert,
		CircleCheck,
		CirclePlay,
		Clock,
		Hourglass,
		TriangleAlertIcon
	} from 'lucide-svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import TimeframeSelect, {
		buildManualTimeframe,
		runsTimeframes,
		useSyncedTimeframe
	} from './runs/TimeframeSelect.svelte'
	import FilterSearchbar, { useUrlSyncedFilterInstance } from './FilterSearchbar.svelte'
	import { jobTriggerKinds } from './triggers/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonMore from './common/toggleButton-v2/ToggleButtonMore.svelte'

	interface Props {
		/** Initial path from route params (e.g., /runs/u/user/script) */
		initialPath?: string
	}

	let paths: string[] = $state([])
	let usernames: string[] = $state([])
	let folders: string[] = $state([])
	let runsFilterSearchbarSchema = $derived(
		buildRunsFilterSearchbarSchema({
			paths,
			usernames,
			folders,
			jobTriggerKinds,
			isSuperAdmin: !!$superadmin,
			isAdminsWorkspace: $workspaceStore === 'admins'
		})
	)
	let perPage = useLocalStorageValue('runs_per_page', 1000, 'number')
	let filters = useUrlSyncedFilterInstance(untrack(() => runsFilterSearchbarSchema))

	let { initialPath }: Props = $props()

	let batchRerunOptionsIsOpen = $state(false)

	// Initialize path filter from route param if provided and not already set via query params
	if (initialPath && !filters.val.path) {
		filters.val.path = initialPath
	}

	let selectedIds: string[] = $state([])
	let selectedWorkspace: string | undefined = $state(undefined)

	let jobKinds: string | undefined = $derived(computeJobKinds(filters.val.job_kinds ?? null))
	let argError = $state('')
	let resultError = $state('')
	let autoRefresh = useLocalStorageValue('auto_refresh_in_runs', true, 'boolean')
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

	let _timeframe = useSyncedTimeframe(
		runsTimeframes,
		() => ({
			maxTs: filters.val.max_ts?.toISOString(),
			minTs: filters.val.min_ts?.toISOString(),
			timeframe: filters.val.timeframe
		}),
		(v) => {
			v.maxTs ? (filters.val.max_ts = new Date(v.maxTs)) : delete filters.val.max_ts
			v.minTs ? (filters.val.min_ts = new Date(v.minTs)) : delete filters.val.min_ts
			v.timeframe ? (filters.val.timeframe = v.timeframe) : delete filters.val.timeframe
		}
	)
	let timeframe = $derived(_timeframe.val)

	let manualTimeframe = $derived(timeframe.type === 'manual' ? timeframe : undefined)

	let graph: 'RunChart' | 'ConcurrencyChart' = $state(
		typeOfChart(page.url.searchParams.get('graph'))
	)
	let innerWidth = $state(window.innerWidth)
	let jobsLoader = useJobsLoader(() => ({
		filters: filters.val,
		timeframe,
		jobKinds,
		autoRefresh: autoRefresh.val,
		argError,
		resultError,
		perPage: perPage.val,
		lookback: graph === 'RunChart' ? 0 : lookback,
		onSetPerPage: (p) => (perPage.val = p),
		currentWorkspace: $workspaceStore ?? ''
	}))
	let lastFetchWentToEnd = $derived(jobsLoader.lastFetchWentToEnd)
	let queue_count = $derived(jobsLoader.queue_count)
	let suspended_count = $derived(jobsLoader.suspended_count)
	let externalJobs = $derived(jobsLoader.externalJobs)
	let extendedJobs = $derived(jobsLoader.extendedJobs)
	// Avoid flicker, but still show empty if loading takes too long
	let debouncedCompletedJobs = new StaleWhileLoading(() => jobsLoader.completedJobs)
	let debouncedJobs = new StaleWhileLoading(() => jobsLoader.jobs)
	let completedJobs = $derived(jobsLoader.completedJobs ?? debouncedCompletedJobs.current)
	let jobs = $derived(jobsLoader.jobs ?? debouncedJobs.current)

	let runsTable: RunsTable | undefined = $state(undefined)

	function reset() {
		_timeframe.val = { ...runsTimeframes[0] }
		selectedIds = []
		delete filters.val.schedule_path
		selectedWorkspace = undefined
		jobsLoader?.loadJobs(true)
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

	function resetAndFilterBy(setter: (string) => void) {
		return (e: CustomEvent<string>) => {
			delete filters.val.path
			delete filters.val.user
			delete filters.val.folder
			delete filters.val.label
			delete filters.val.concurrency_key
			delete filters.val.tag
			delete filters.val.worker
			delete filters.val.schedule_path
			setter(e.detail)
		}
	}

	const filterByPath = resetAndFilterBy((s) => (filters.val.path = s))
	const filterByUser = resetAndFilterBy((s) => (filters.val.user = s))
	const filterByFolder = resetAndFilterBy((s) => (filters.val.folder = s))
	const filterByLabel = resetAndFilterBy((s) => (filters.val.label = s))
	const filterByConcurrencyKey = resetAndFilterBy((s) => (filters.val.concurrency_key = s))
	const filterByTag = resetAndFilterBy((s) => (filters.val.tag = s))
	const filterBySchedule = resetAndFilterBy((s) => (filters.val.schedule_path = s))
	const filterByWorker = resetAndFilterBy((s) => (filters.val.worker = s))

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

	function getSelectedFilters() {
		const argFilter = filters.val.arg && JSON.parse(filters.val.arg)
		const resultFilter = filters.val.result && JSON.parse(filters.val.result)
		const { minTs, maxTs } = timeframe.computeMinMax()
		return {
			workspace: $workspaceStore ?? '',
			startedBefore: maxTs ?? undefined,
			startedAfter: minTs ?? undefined,
			schedulePath: filters.val.schedule_path ?? undefined,
			scriptPathExact:
				filters.val.path === null || filters.val.path === '' ? undefined : filters.val.path,
			createdBy: filters.val.user || undefined,
			scriptPathStart: filters.val.folder ? `f/${filters.val.folder}/` : undefined,
			jobKinds: jobKinds == '' ? undefined : jobKinds,
			success:
				filters.val.status == 'success'
					? true
					: filters.val.status == 'failure'
						? false
						: undefined,
			running:
				filters.val.status == 'running' || filters.val.status == 'suspended'
					? true
					: filters.val.status == 'waiting'
						? false
						: undefined,
			isSkipped: filters.val.show_skipped ? undefined : false,
			// isFlowStep: jobKindsCat != 'all' ? false : undefined,
			hasNullParent:
				filters.val.path != undefined ||
				filters.val.path != undefined ||
				filters.val.job_kinds != 'all'
					? true
					: undefined,
			label: filters.val.label || undefined,
			tag: filters.val.tag || undefined,
			suspended:
				filters.val.status == 'waiting'
					? false
					: filters.val.status == 'suspended'
						? true
						: undefined,
			scheduledForBeforeNow:
				filters.val.show_future_jobs == false ||
				filters.val.status == 'waiting' ||
				filters.val.status == 'suspended'
					? true
					: undefined,
			args: argFilter && Object.keys(argFilter).length && !argError ? argFilter : undefined,
			result:
				resultFilter && Object.keys(resultFilter).length && !resultError ? resultFilter : undefined,
			jobTriggerKind: filters.val.job_trigger_kind || undefined,
			allWorkspaces: filters.val.all_workspaces || undefined,
			allowWildcards: allowWildcards(filters.val) || undefined
		}
	}
	async function cancelJobs(uuidsToCancel: string[], forceCancel: boolean = false) {
		const uuids = await JobService.cancelSelection({
			workspace: $workspaceStore ?? '',
			requestBody: uuidsToCancel,
			forceCancel: forceCancel
		})
		selectedIds = []
		jobsLoader?.loadJobs(true, true)
		sendUserToast(`Canceled ${uuids.length} jobs`)
	}

	async function onCancelAllJobsMatchingFilters() {
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

	async function onCancelSelectedJobs(jobIdsToCancel: string[]) {
		forceCancelInPopup = true
		askingForConfirmation = {
			confirmBtnText: `Cancel ${jobIdsToCancel.length} jobs`,
			title: `Confirm cancelling ${jobIdsToCancel.length} jobs`,
			onConfirm: (forceCancel) => {
				cancelJobs(jobIdsToCancel, forceCancel)
			}
		}
	}

	async function reRunJobs(jobIdsToReRun: string[], batchReRunOptions: BatchReRunOptions) {
		if (!$workspaceStore) return

		if (askingForConfirmation) {
			askingForConfirmation.loading = true
		}

		const body: Parameters<typeof JobService.batchReRunJobs>[0]['requestBody'] = {
			job_ids: jobIdsToReRun,
			script_options_by_path: batchReRunOptions.script ?? {},
			flow_options_by_path: batchReRunOptions.flow ?? {}
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
		jobsLoader?.loadJobs(true, true)
	}

	async function onRerunAllJobsMatchingFilters() {
		const selectedFilters = getSelectedFilters()
		selectedIds = []

		const loadingToast = sendUserToast('Loading job ids', 'info')

		if (filters.val.job_kinds !== 'runs') {
			sendUserToast('Batch re-run is only supported for scripts and flows', true)
			loadingToast.destroy()
			return
		}
		selectedIds = await JobService.listFilteredJobsUuids({
			...selectedFilters,
			jobKinds: 'script,flow'
		})
		loadingToast.destroy()
		batchRerunOptionsIsOpen = true
	}

	async function onReRunSelectedJobs(batchReRunOptions: BatchReRunOptions) {
		const jobIdsToReRun = selectedIds
		askingForConfirmation = {
			title: `Confirm re-running the selected jobs`,
			confirmBtnText: `Re-run ${jobIdsToReRun.length} jobs`,
			type: 'reload',
			onConfirm: async () => {
				await reRunJobs(jobIdsToReRun, batchReRunOptions)
			}
		}
	}

	async function loadExtra() {
		await jobsLoader?.loadExtraJobs()
	}

	function jobsFilter(f: 'waiting' | 'suspended') {
		delete filters.val.path
		delete filters.val.user
		delete filters.val.folder
		delete filters.val.label
		delete filters.val.concurrency_key
		delete filters.val.tag
		delete filters.val.worker
		delete filters.val.schedule_path
		filters.val.status = filters.val.status == f ? undefined : f
		filters.val.job_kinds = 'all'
	}

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

	const smallScreenWidth = 1920

	let forceCancelInPopup = $state(false)

	const warnJobLimitMsg = $derived(
		`The exact number of concurrent jobs at the beginning of the time range may be incorrect as only the last ${perPage.val} jobs are taken into account: a job that was started earlier than this limit will not be taken into account`
	)

	let manualSelectionMode: undefined | 'cancel' | 'rerun' = $state()
</script>

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
				<JobRunsPreview id={selectedIds[0]} workspace={selectedWorkspace} />
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
		<div class="flex flex-row items-start w-full px-4 gap-3 py-4 flex-wrap">
			<div class="flex flex-row items-center gap-6">
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

				<!-- Queue -->
				<RunsQueue
					success={filters.val.status ?? null}
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

			<div class="hidden xl:flex gap-2 ml-6">
				<ToggleButtonGroup
					tabListClass="hidden 2xl:flex"
					bind:selected={
						() => filters.val.job_kinds ?? 'runs',
						(v) => (v === 'runs' ? delete filters.val.job_kinds : (filters.val.job_kinds = v))
					}
				>
					{#snippet children({ item })}
						<ToggleButton value="all" label="All" {item} />
						<ToggleButton
							value="runs"
							label="Runs"
							showTooltipIcon
							tooltip="Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook"
							{item}
						/>
						<ToggleButton
							value="dependencies"
							label="Deps"
							showTooltipIcon
							tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
							{item}
						/>
						<ToggleButtonMore
							togglableItems={[
								{
									label: 'Previews',
									value: 'previews',
									tooltip: "Previews are jobs that have been started in the editor as 'Tests'"
								},
								{
									label: 'Sync',
									value: 'deploymentcallbacks',
									tooltip:
										'Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings'
								}
							]}
							{item}
							bind:selected={filters.val.job_kinds}
						/>
					{/snippet}
				</ToggleButtonGroup>
				<ToggleButtonGroup
					bind:selected={
						() => filters.val.status ?? 'all',
						(v) => (v === 'all' ? delete filters.val.status : (filters.val.status = v))
					}
					id="status"
				>
					{#snippet children({ item })}
						<ToggleButton value={'all'} label="All" {item} />
						<ToggleButton
							value={'running'}
							tooltip="Running"
							class="whitespace-nowrap"
							icon={CirclePlay}
							iconProps={{
								class:
									'group-data-[state=on]:text-yellow-600 dark:group-data-[state=on]:text-yellow-400'
							}}
							{item}
						/>
						<ToggleButton
							value={'success'}
							tooltip="Success"
							class="whitespace-nowrap"
							icon={CircleCheck}
							iconProps={{
								class:
									'group-data-[state=on]:text-green-500 dark:group-data-[state=on]:text-green-300'
							}}
							{item}
						/>
						<ToggleButton
							value={'failure'}
							tooltip="Failure"
							class="whitespace-nowrap"
							icon={CircleAlert}
							iconProps={{
								class: 'group-data-[state=on]:text-red-500 dark:group-data-[state=on]:text-red-300'
							}}
							{item}
						/>
						{#if filters.val.status == 'waiting'}
							<ToggleButton
								value={'waiting'}
								tooltip="Waiting"
								class="whitespace-nowrap"
								icon={Hourglass}
								selectedColor="blue"
								{item}
							/>
						{:else if filters.val.status == 'suspended'}
							<ToggleButton
								value={'suspended'}
								tooltip="Suspended"
								class="whitespace-nowrap"
								icon={Hourglass}
								selectedColor="purple"
								{item}
							/>
						{/if}
					{/snippet}
				</ToggleButtonGroup>
			</div>

			<div class="hidden xl:flex gap-2 items-center min-h-8 ml-2">
				{#if !filters.val.job_trigger_kind || filters.val.job_trigger_kind === '!schedule'}
					<div class="flex items-center gap-1" title="Show schedules">
						<Toggle
							size="xs"
							color="nord"
							id="show-schedules"
							bind:checked={
								() => filters.val.job_trigger_kind !== '!schedule',
								(v) =>
									v
										? delete filters.val.job_trigger_kind
										: (filters.val.job_trigger_kind = '!schedule')
							}
						/>
						<Calendar size={14} />
					</div>
				{/if}
				<div class="flex items-center gap-1" title="Show future jobs">
					<Toggle
						size="xs"
						color="nord"
						id="show-future-jobs"
						bind:checked={
							() => filters.val.show_future_jobs !== false,
							(v) =>
								v ? delete filters.val.show_future_jobs : (filters.val.show_future_jobs = false)
						}
					/>
					<Clock size={14} />
				</div>
			</div>

			<TimeframeSelect
				wrapperClasses="ml-auto"
				onClick={() => jobsLoader?.loadJobs(true)}
				loading={jobsLoader?.loading}
				items={runsTimeframes}
				bind:value={_timeframe.val}
			/>
			<FilterSearchbar
				class={twMerge(
					'flex-1 relative min-w-[18rem]',
					Object.keys(filters.val).length <= 3 ? 'max-w-[28rem]' : 'max-w-[34rem]',
					ButtonType.UnifiedMinHeightClasses.md
				)}
				schema={runsFilterSearchbarSchema}
				presets={buildRunsFilterPresets({
					isSuperadmin: !!$superadmin,
					isAdminsWorkspace: $workspaceStore === 'admins'
				})}
				bind:value={filters.val}
				placeholder="Filter runs..."
			/>
		</div>

		<!-- Graph -->
		<div class="p-2 px-4 bg-surface-tertiary mx-4 border rounded-md">
			<div class="relative z-10 mb-2 flex gap-2">
				<Tabs bind:selected={graph}>
					<Tab value="RunChart" label="Duration" />
					<Tab value="ConcurrencyChart" label="Concurrency">
						{#snippet extra()}
							{#if warnJobLimit}
								<Tooltip Icon={TriangleAlertIcon}>{warnJobLimitMsg}</Tooltip>
							{/if}
						{/snippet}
					</Tab>
				</Tabs>

				{#if graph !== 'RunChart'}
					<Select
						class="ml-2"
						bind:value={lookback}
						items={[
							{ label: 'None', value: 0 },
							{ label: '1 day', value: 1 },
							{ label: '3 days', value: 3 },
							{ label: '7 days', value: 7 }
						]}
						transformInputSelectedText={(_, v) => `${pluralize(v, 'day')} lookback`}
						tooltip={'How far behind the min datetime to start considering jobs for the concurrency graph. Change this value to include jobs started before the set time window for the computation of the graph'}
					/>
				{:else if !lastFetchWentToEnd && (jobs?.length ?? 0) >= (perPage.val ?? 1000)}
					<Button wrapperClasses="ml-2" unifiedSize="md" onClick={() => jobsLoader.loadExtraJobs()}>
						Load more
						<Tooltip>There are more jobs to load</Tooltip>
					</Button>
				{/if}
			</div>
			{#if graph === 'RunChart'}
				{@const manualTimeframe = timeframe.type === 'manual' ? timeframe : undefined}
				<RunChart
					bind:selectedIds
					canSelect
					minTimeSet={manualTimeframe?.minTs}
					maxTimeSet={manualTimeframe?.maxTs}
					maxIsNow={manualTimeframe?.maxTs == undefined}
					jobs={completedJobs}
					onZoom={(zoom) => {
						_timeframe.val = buildManualTimeframe(zoom.min.toISOString(), zoom.max.toISOString())
					}}
					onPointClicked={(ids) => {
						runsTable?.scrollToRun(ids)
					}}
				/>
			{:else if graph === 'ConcurrencyChart'}
				<ConcurrentJobsChart
					minTimeSet={manualTimeframe?.minTs}
					maxTimeSet={manualTimeframe?.maxTs}
					maxIsNow={manualTimeframe?.maxTs == undefined}
					{extendedJobs}
					onZoom={(zoom) => {
						_timeframe.val = buildManualTimeframe(zoom.min.toISOString(), zoom.max.toISOString())
					}}
				/>
			{/if}
		</div>

		<div class="grow min-h-0 bottom-splitpane-wrapper">
			<Splitpanes>
				<Pane minSize={40}>
					<div class="h-full flex">
						<div class="flex flex-col flex-1 m-4 mt-2 mr-2">
							<!-- Runs table. Add overflow-hidden because scroll is handled inside the runs table based on this wrapper height -->
							<div class="grow min-h-0 overflow-y-hidden overflow-x-auto">
								{#if jobs}
									<RunsTable
										{jobs}
										externalJobs={externalJobs ?? []}
										omittedObscuredJobs={extendedJobs?.omitted_obscured_jobs ?? false}
										showExternalJobs={graph !== 'RunChart'}
										activeLabel={filters.val.label}
										{lastFetchWentToEnd}
										bind:selectedIds
										bind:selectedWorkspace
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
										perPage={perPage.val}
										bind:batchRerunOptionsIsOpen
										onCancelJobs={onCancelSelectedJobs}
										{manualSelectionMode}
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
								class="bg-surface-tertiary border rounded-b-md flex text-xs px-2 py-1 items-center gap-4"
							>
								{#if !manualSelectionMode}
									<DropdownV2
										btnText="Batch actions"
										size="xs"
										items={[
											{
												displayName: 'Cancel jobs',
												action: () => ((manualSelectionMode = 'cancel'), (selectedIds = []))
											},
											{
												displayName: 'Re-run jobs',
												action: () => {
													manualSelectionMode = 'rerun'
													selectedIds = []
													batchRerunOptionsIsOpen = true
												}
											},
											{
												displayName: 'Cancel all jobs matching filters',
												action: () => onCancelAllJobsMatchingFilters()
											},
											{
												displayName: 'Re-run all jobs matching filters',
												action: () => onRerunAllJobsMatchingFilters()
											}
										]}
									/>
								{:else}
									<Button
										size="xs"
										destructive
										onClick={() => {
											manualSelectionMode = undefined
											batchRerunOptionsIsOpen = false
										}}
									>
										Exit selection mode
									</Button>
								{/if}
								<div class="flex-1"></div>
								<Toggle
									size="xs"
									color="nord"
									bind:checked={autoRefresh.val}
									options={{ right: 'Auto-refresh' }}
									textClass="whitespace-nowrap"
								/>
								<Select
									class="w-24"
									bind:value={
										() => perPage.val,
										(newPerPage) => {
											perPage.val = newPerPage
											if (newPerPage > (jobs?.length ?? 1000)) loadExtra()
										}
									}
									onCreateItem={(v) => (perPage.val = parseInt(v))}
									items={[
										{ value: 25, label: '25' },
										{ value: 100, label: '100' },
										{ value: 1000, label: '1000' },
										{ value: 10000, label: '10000' }
									]}
									transformInputSelectedText={(_, v) => `${v} / page`}
								/>
							</div>
						</div>
					</div>
				</Pane>
				<AnimatedPane
					size={40}
					minSize={15}
					class="flex flex-col"
					opened={selectedIds.length > 0 || !!manualSelectionMode}
				>
					<div class="mt-12 overflow-y-auto pr-4 ml-2 relative flex-1">
						{#if manualSelectionMode === 'cancel'}
							<div
								class="rounded-md bg-surface-tertiary border absolute inset-0 mb-4 flex flex-col items-center justify-center"
							>
								<Button
									destructive
									variant="accent"
									disabled={!selectedIds.length}
									onClick={() => onCancelSelectedJobs(selectedIds)}
								>
									Cancel {selectedIds.length} jobs
								</Button>
							</div>
						{:else if batchRerunOptionsIsOpen}
							<BatchReRunOptionsPane
								{selectedIds}
								onCancel={() => (
									(batchRerunOptionsIsOpen = false),
									(manualSelectionMode = undefined)
								)}
								onConfirm={async (options) => {
									await onReRunSelectedJobs(options)
								}}
							/>
						{:else if selectedIds.length === 1}
							{#if selectedIds[0] === '-'}
								<div class="p-4">There is no information available for this job</div>
							{:else}
								<JobRunsPreview
									id={selectedIds[0]}
									workspace={selectedWorkspace}
									on:filterByConcurrencyKey={filterByConcurrencyKey}
									on:filterByWorker={filterByWorker}
								/>
							{/if}
						{:else if selectedIds.length > 1}
							<div
								class="rounded-md bg-surface-tertiary border absolute inset-0 mb-4 flex items-center justify-center"
							>
								<div class="text-xs m-4"> {selectedIds.length} jobs selected</div>
							</div>
						{/if}
					</div>
				</AnimatedPane>
			</Splitpanes>
		</div>
	</div>
{/if}

<style>
	:global(.bottom-splitpane-wrapper .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		/* opacity: 0 !important; */
	}
</style>
