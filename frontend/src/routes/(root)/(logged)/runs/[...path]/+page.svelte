<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import { JobService, Job, CompletedJob, ScriptService, FlowService } from '$lib/gen'
	import { setQueryWithoutLoad } from '$lib/utils'

	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import { goto } from '$app/navigation'
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

	let jobs: Job[] | undefined
	let intervalId: NodeJS.Timer | undefined
	let selectedId: string | undefined = undefined

	let success: boolean | undefined =
		$page.url.searchParams.get('success') != undefined
			? $page.url.searchParams.get('success') == 'true'
			: undefined
	let isSkipped: boolean | undefined =
		$page.url.searchParams.get('is_skipped') != undefined
			? $page.url.searchParams.get('is_skipped') == 'true'
			: false

	let argFilter: any = $page.url.searchParams.get('arg') ?? undefined
	let resultFilter: any = $page.url.searchParams.get('result') ?? undefined
	let minTs = $page.url.searchParams.get('min_ts') ?? undefined
	let maxTs = $page.url.searchParams.get('max_ts') ?? undefined
	let schedulePath = $page.url.searchParams.get('schedule_path') ?? undefined

	let nbObJobs = 30

	$: path = $page.params.path
	$: jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'
	$: jobKinds = computeJobKinds(jobKindsCat)

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == 'all') {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES},${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW},${CompletedJob.job_kind.SCRIPT_HUB}`
		} else if (jobKindsCat == 'dependencies') {
			return `${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.FLOWDEPENDENCIES}`
		} else if (jobKindsCat == 'previews') {
			return `${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW}`
		} else {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW}`
		}
	}

	$: ($workspaceStore && loadJobs()) || (path && success && isSkipped && jobKinds)

	async function fetchJobs(
		startedBefore: string | undefined,
		startedAfter: string | undefined
	): Promise<Job[]> {
		return JobService.listJobs({
			workspace: $workspaceStore!,
			startedBefore,
			startedAfter,
			schedulePath,
			scriptPathExact: path === '' ? undefined : path,
			jobKinds,
			success,
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
		jobs = undefined
		try {
			const newJobs = await fetchJobs(maxTs, minTs)
			jobs = newJobs
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			console.error(JSON.stringify(err))
		}
		loading = false
	}

	async function syncer() {
		if (sync && jobs && maxTs == undefined) {
			const reversedJobs = [...jobs].reverse()
			const lastIndex = reversedJobs.findIndex((x) => x.type == Job.type.QUEUED_JOB) - 1
			let ts = lastIndex >= 0 ? reversedJobs[lastIndex].created_at : undefined
			if (!ts) {
				const date = jobs.length > 0 ? new Date(jobs[0]?.created_at!) : new Date()
				date.setSeconds(date.getSeconds() + 1)
				ts = date.toISOString()
			}
			const newJobs = await fetchJobs(maxTs, minTs ?? ts)
			if (newJobs && newJobs.length > 0 && jobs) {
				const oldJobs = jobs?.map((x) => x.id)
				jobs = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
				newJobs
					.filter((x) => oldJobs.includes(x.id))
					.forEach((x) => (jobs![jobs?.findIndex((y) => y.id == x.id)!] = x))
				jobs = jobs
			}
		}
	}

	let sync = true
	let mounted: boolean = false
	onMount(() => {
		mounted = true
		loadPaths()
		intervalId = setInterval(syncer, 5000)

		document.addEventListener('visibilitychange', () => {
			if (document.hidden) {
				sync = false
			} else {
				sync = true
			}
		})
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

	async function loadPaths() {
		const npaths_scripts = await ScriptService.listScriptPaths({ workspace: $workspaceStore ?? '' })
		const npaths_flows = await FlowService.listFlowPaths({ workspace: $workspaceStore ?? '' })
		paths = npaths_scripts.concat(npaths_flows).sort()
	}

	async function syncTsWithURL(minTs?: string, maxTs?: string) {
		setQueryWithoutLoad($page.url, [
			{ key: 'min_ts', value: minTs },
			{ key: 'max_ts', value: maxTs }
		])
	}

	$: syncTsWithURL(minTs, maxTs)

	$: completedJobs =
		jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []

	let searchPath = ''
	$: searchPath = path

	$: searchPath && onSearchPathChange()

	function onSearchPathChange() {
		goto(`/runs/${searchPath}?${$page.url.searchParams.toString()}`)
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
			label: 'Last 30 seconds',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last minute',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last 5 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 5 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last 30 minutes',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last 24 hours',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last 7 days',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		},
		{
			label: 'Last month',
			setMinMax: () => {
				minTs = new Date(new Date().getTime() - 30 * 24 * 60 * 60 * 1000).toISOString()
				maxTs = new Date().toISOString()
			}
		}
	]

	let selectedManualDate = 0
	let autoRefresh: boolean = true
	let runDrawer: Drawer
</script>

<div class="w-full h-screen">
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
			<div class="hidden xl:block">
				<RunsFilter
					bind:isSkipped
					{paths}
					{jobKindsCat}
					bind:selectedPath={searchPath}
					bind:success
					bind:argFilter
					bind:resultFilter
					bind:argError
					bind:resultError
					on:change={reloadLogsWithoutFilterError}
					on:clearFilters={() => {
						minTs = undefined
						maxTs = undefined
						autoRefresh = true
					}}
				/>
			</div>
			<div class="xl:hidden">
				<MobileFilters>
					<svelte:fragment slot="filters">
						<span class="text-xs font-semibold leading-6">Filters</span>
						<RunsFilter
							bind:isSkipped
							{paths}
							{jobKindsCat}
							bind:selectedPath={searchPath}
							bind:success
							bind:argFilter
							bind:resultFilter
							bind:argError
							bind:resultError
							on:change={reloadLogsWithoutFilterError}
							on:clearFilters={() => {
								minTs = undefined
								maxTs = undefined
								autoRefresh = true
							}}
						/>
					</svelte:fragment>
				</MobileFilters>
			</div>
		</div>
	</div>

	<div class="p-2 w-full">
		<RunChart
			jobs={completedJobs}
			on:zoom={async (e) => {
				minTs = e.detail.min.toISOString()
				maxTs = e.detail.max.toISOString()
				jobs = await fetchJobs(maxTs, minTs)
			}}
		/>
	</div>
	<div class="flex flex-col gap-1 md:flex-row w-full p-4 justify-between">
		<div class="flex flex-row gap-1 w-full">
			<div class="relative w-full">
				<div class="flex gap-1 relative w-full">
					<span class="text-xs absolute -top-4">Min datetime</span>

					<input type="text" value={minTs ?? 'zoom x axis to set min (drag with ctrl)'} disabled />

					<CalendarPicker
						date={minTs}
						label="Min datetimes"
						on:change={async ({ detail }) => {
							minTs = new Date(detail).toISOString()
							jobs = await fetchJobs(maxTs, minTs)
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
							jobs = await fetchJobs(maxTs, minTs)
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

	<SplitPanesWrapper class="hidden md:block">
		<Splitpanes>
			<Pane size={60} minSize={50}>
				{#if jobs}
					<RunsTable
						{jobs}
						bind:selectedId
						bind:nbObJobs
						loadMoreQuantity={30}
						on:filterByPath={(e) => {
							searchPath = e.detail
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
					{#key selectedId}
						<JobPreview id={selectedId} />
					{/key}
				{:else}
					<div class="text-xs m-4">No job selected</div>
				{/if}
			</Pane>
		</Splitpanes>
	</SplitPanesWrapper>

	<div class="md:hidden">
		{#if jobs}
			<RunsTable
				{jobs}
				bind:selectedId
				bind:nbObJobs
				loadMoreQuantity={30}
				on:select={() => {
					runDrawer.openDrawer()
				}}
			/>
		{/if}
	</div>
</div>

<Drawer bind:this={runDrawer}>
	<DrawerContent title="Run details" on:close={runDrawer.closeDrawer}>
		{#if selectedId}
			<JobPreview id={selectedId} />
		{/if}
	</DrawerContent>
</Drawer>
