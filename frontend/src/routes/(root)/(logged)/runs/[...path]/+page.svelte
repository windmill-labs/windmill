<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import { JobService, Job, CompletedJob, ScriptService, FlowService } from '$lib/gen'
	import { setQuery, setQueryWithoutLoad } from '$lib/utils'

	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import JobDetail from '$lib/components/jobs/JobDetail.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import { goto } from '$app/navigation'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import RunChart from '$lib/components/RunChart.svelte'
	import { faSearchMinus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import AutoComplete from 'simple-svelte-autocomplete'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import Slider from '$lib/components/Slider.svelte'
	import JsonEditor from '$lib/components/apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'

	let jobs: Job[] | undefined
	let error: Error | undefined
	let intervalId: NodeJS.Timer | undefined

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

	let nbOfJobs = 30

	$: path = $page.params.path

	$: jobKindsCat = $page.url.searchParams.get('job_kinds') ?? 'runs'

	$: jobKinds = computeJobKinds(jobKindsCat)

	function computeJobKinds(jobKindsCat: string | undefined): string {
		if (jobKindsCat == 'all') {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.DEPENDENCIES},${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW},${CompletedJob.job_kind.SCRIPT_HUB}`
		} else if (jobKindsCat == 'dependencies') {
			return CompletedJob.job_kind.DEPENDENCIES
		} else if (jobKindsCat == 'previews') {
			return `${CompletedJob.job_kind.PREVIEW},${CompletedJob.job_kind.FLOWPREVIEW}`
		} else {
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW}`
		}
	}

	$: ($workspaceStore && loadJobs()) || (path && success && isSkipped && jobKinds)

	let filterTimeout: NodeJS.Timeout | undefined = undefined
	function debounceSyncer() {
		if (resultError == '' && argError == '') {
			filterTimeout && clearTimeout(filterTimeout)
			filterTimeout = setTimeout(() => {
				loadJobs()
			}, 2000)
		}
	}

	async function fetchJobs(
		startedBefore: string | undefined,
		startedAfter: string | undefined
	): Promise<Job[]> {
		return JobService.listJobs({
			workspace: $workspaceStore!,
			startedBefore,
			startedAfter,
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

	async function loadJobs(): Promise<void> {
		jobs = undefined
		try {
			const newJobs = await fetchJobs(maxTs, minTs)
			jobs = newJobs
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			error = err
			console.error(JSON.stringify(err))
		}
	}

	async function loadOlderJobs() {
		if (jobs) {
			nbOfJobs += 30
		}
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
	onMount(() => {
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

	let paths: string[] = []

	async function loadPaths() {
		const npaths_scripts = await ScriptService.listScriptPaths({ workspace: $workspaceStore ?? '' })
		const npaths_flows = await FlowService.listFlowPaths({ workspace: $workspaceStore ?? '' })
		paths = npaths_scripts.concat(npaths_flows).sort()
	}

	function syncWithUrl(arg: string, value: string) {
		setQueryWithoutLoad($page.url, [{ key: arg, value }])
	}

	async function syncTsWithURL(minTs?: string, maxTs?: string) {
		setQueryWithoutLoad($page.url, [
			{ key: 'min_ts', value: minTs },
			{ key: 'max_ts', value: maxTs }
		])
	}

	$: syncWithUrl('job_kinds', jobKindsCat)
	$: syncWithUrl('arg', argFilter)
	$: syncWithUrl('result', resultFilter)
	$: syncTsWithURL(minTs, maxTs)

	$: completedJobs =
		jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
	let searchPath = ''
	$: searchPath = path

	$: searchPath && onSearchPathChange()

	function onSearchPathChange() {
		goto(`/runs/${searchPath}?${$page.url.searchParams.toString()}`)
	}

	let argError = ''
	let resultError = ''
</script>

<CenteredPage>
	<PageHeader
		title="Runs {path ? `of ${path}` : ''}"
		tooltip="All past and schedule executions of scripts and flows, including previews.
	You only see your own runs or runs of groups you belong to unless you are an admin."
	/>

	<div class="max-w-7x mt-2">
		<div class="flex flex-row space-x-4">
			<select
				bind:value={success}
				on:change={async () => await setQuery($page.url, 'success', String(success))}
			>
				<option value={undefined}>Successful and error jobs</option>
				<option value={true}>Only successful jobs</option>
				<option value={false}>Only error jobs</option>
			</select>
			<select
				bind:value={isSkipped}
				on:change={async () => await setQuery($page.url, 'is_skipped', String(isSkipped))}
			>
				<option value={false}>Ignore skipped flow jobs</option>
				<option value={true}>If a flow job, show only if it was not skipped</option>
				<option value={undefined}>Show flow jobs regardless of being skipped or not</option>
			</select>
		</div>
		<div>
			<div class="my-2 pb-2">
				<Tabs
					selected={jobKindsCat}
					on:selected={(e) => {
						const url = new URL($page.url)
						url.searchParams.set('job_kinds', e.detail)
						goto(url)
					}}
				>
					<Tab value="all">All</Tab>
					<Tab value="runs">Runs</Tab>
					<Tab value="previews">Previews</Tab>
					<Tab value="dependencies">Dependencies</Tab>
				</Tabs>
			</div>
			<div class="border mb-4">
				<RunChart
					jobs={completedJobs}
					on:zoom={async (e) => {
						minTs = e.detail.min.toISOString()
						maxTs = e.detail.max.toISOString()
						jobs = await fetchJobs(maxTs, minTs)
					}}
				/>
			</div>
			<div class="flex flex-row gap-x-2 w-full mb-2">
				<div class="relative w-full"
					><span class="text-xs absolute -top-4">min datetime</span>
					<input type="text" value={minTs ?? 'zoom x axis to set min (drag with ctrl)'} disabled />
				</div>
				<div class="relative w-full"
					><span class="text-xs absolute -top-4">max datetime</span>
					<input type="text" value={maxTs ?? 'zoom x axis to set max'} disabled />
				</div>
			</div>
			<div class="grid grid-cols-1 md:grid-cols-3 gap-2 mb-2 w-full flex-wrap">
				<div>
					<div class="flex flex-row gap-x-2">
						{#key path}
							<AutoComplete
								items={paths}
								value={path}
								bind:selectedItem={searchPath}
								placeholder="Search by path"
							/>
						{/key}
						<Button
							title="Clear path and time filters"
							variant="border"
							on:click={async () => {
								minTs = undefined
								maxTs = undefined
								jobs = undefined
								await goto('/runs?' + $page.url.searchParams.toString())
								loadJobs()
							}}
							size="xs"
						>
							<Icon data={faSearchMinus} />
						</Button>
					</div>
				</div>
				<div
					><Slider
						text="Filter by args {argFilter ? '(set)' : ''}"
						tooltip={'Filter by a json being a subset of the args. Try \'{"foo": "bar"}\''}
						><JsonEditor
							on:change={debounceSyncer}
							bind:error={argError}
							bind:code={argFilter}
						/></Slider
					></div
				>
				<div
					><Slider
						text="Filter by result {resultFilter ? '(set)' : ''}"
						tooltip={'Filter by a json being a subset of the result. Try \'{"foo": "bar"}\''}
						><JsonEditor
							on:change={debounceSyncer}
							bind:error={resultError}
							bind:code={resultFilter}
						/></Slider
					></div
				>
			</div>

			<Skeleton loading={!jobs} layout={[[6], 1, [6], 1, [6], 1, [6], 1, [6]]} />

			{#if jobs}
				<div class="space-y-0">
					{#each jobs.slice(0, nbOfJobs) as job (job.id)}
						<JobDetail {job} />
						<div class="line w-20 h-4" />
					{/each}
				</div>
				{#if jobs.length == 0}
					<NoItemFound />
				{/if}
			{/if}
		</div>
		{#if error}
			<div>
				{JSON.stringify(error)}
			</div>
		{/if}
	</div>

	{#if (jobs?.length ?? 0) >= nbOfJobs}
		<div class="text-center pb-6">
			<button class=" mt-4 text-blue-500 text-center text-sm" on:click={loadOlderJobs}>
				Load older jobs
			</button>
		</div>
	{/if}
</CenteredPage>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #999 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
