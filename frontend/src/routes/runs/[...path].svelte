<script context="module">
	export function load() {
		return {
			stuff: { title: 'Runs' }
		}
	}
</script>

<script lang="ts">
	import { onDestroy } from 'svelte'
	import { JobService, Job, CompletedJob } from '$lib/gen'
	import { setQuery } from '$lib/utils'

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
	import { faSearch, faSearchMinus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	let jobs: Job[] | undefined
	let error: Error | undefined
	let intervalId: NodeJS.Timer | undefined
	let createdBefore: string | undefined = $page.url.searchParams.get('createdBefore') ?? undefined

	let success: boolean | undefined =
		$page.url.searchParams.get('success') != undefined
			? $page.url.searchParams.get('success') == 'true'
			: undefined
	let isSkipped: boolean | undefined =
		$page.url.searchParams.get('is_skipped') != undefined
			? $page.url.searchParams.get('is_skipped') == 'true'
			: false

	let showOlderJobs = true
	const jobsPerPage = 100

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
			return `${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.SCRIPT_HUB}`
		}
	}

	$: ($workspaceStore && loadJobs(createdBefore)) || (success && isSkipped && jobKinds)

	async function fetchJobs(
		createdBefore: string | undefined,
		createdAfter: string | undefined
	): Promise<Job[]> {
		return JobService.listJobs({
			workspace: $workspaceStore!,
			createdBefore,
			createdAfter,
			scriptPathExact: path === '' ? undefined : path,
			jobKinds,
			success,
			isSkipped,
			isFlowStep: jobKindsCat != 'all' ? false : undefined
		})
	}

	async function fetchCompletedJobs(createdBefore: string | undefined): Promise<CompletedJob[]> {
		return JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			createdBefore,
			scriptPathExact: path === '' ? undefined : path,
			jobKinds: jobKinds,
			success,
			isSkipped
		})
	}

	async function loadJobs(createdBefore: string | undefined): Promise<void> {
		try {
			const newJobs = await fetchJobs(createdBefore, undefined)
			showOlderJobs = newJobs.length === jobsPerPage
			jobs = newJobs
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			error = err
			console.error(JSON.stringify(err))
		}
	}

	async function loadOlderJobs() {
		if (jobs) {
			const ts = jobs[jobs.length - 1].created_at
			const olderJobs = await fetchCompletedJobs(ts!)
			showOlderJobs = olderJobs.length === jobsPerPage
			jobs = jobs.concat(...olderJobs)
		}
	}

	async function syncer() {
		if (jobs && createdBefore === undefined) {
			const reversedJobs = jobs.slice(0, jobsPerPage).reverse()
			const lastIndex = reversedJobs.findIndex((x) => x.type == Job.type.QUEUED_JOB) - 1
			let ts = lastIndex >= 0 ? reversedJobs[lastIndex].created_at : undefined
			if (!ts) {
				const date = jobs.length > 0 ? new Date(jobs[0]?.created_at!) : new Date()
				date.setSeconds(date.getSeconds() + 1)
				ts = date.toISOString()
			}
			const newJobs = await fetchJobs(maxTs, minTs ?? ts)
			if (newJobs && newJobs.length > 0) {
				const oldJobs = jobs.map((x) => x.id)
				jobs = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
				newJobs
					.filter((x) => oldJobs.includes(x.id))
					.forEach((x) => (jobs![jobs?.findIndex((y) => y.id == x.id)!] = x))
				jobs = jobs
			}
		}
	}

	$: {
		if ($workspaceStore) {
			loadJobs(createdBefore)
			path // trigger on path change
			success && isSkipped && jobKinds
			if (intervalId) {
				clearInterval(intervalId)
			}
			intervalId = setInterval(syncer, 5000)
		}
	}

	async function syncCatWithURL() {
		await setQuery($page.url, 'job_kinds', jobKindsCat)
	}

	$: jobKindsCat && syncCatWithURL()

	$: completedJobs =
		jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []
	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
	let searchPath = ''
	$: searchPath = path
	let minTs = undefined
	let maxTs = undefined
</script>

<CenteredPage>
	<PageHeader
		title="Runs {path ? `of ${path}` : ''}"
		tooltip="All past and schedule executions of scripts and flows, including previews.
	You only see your own runs or runs of groups you belong to unless you are an admin."
	/>
	<div class="flex flex-row gap-x-2">
		<input placeholder="Search jobs at a given path" type="text" bind:value={searchPath} />
		<Button
			variant="border"
			on:click={() => {
				goto('/runs?' + $page.url.searchParams.toString())
			}}
			size="xs"
		>
			<Icon data={faSearchMinus} />
		</Button>
		<Button
			variant="border"
			on:click={() => {
				goto('/runs?' + $page.url.searchParams.toString())
			}}
			size="xs"
		>
			<Icon data={faSearch} />
		</Button>
	</div>

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
				<Button
					variant="border"
					on:click={async () => {
						minTs = undefined
						maxTs = undefined
						jobs = await fetchJobs(maxTs, minTs)
					}}
					size="xs"
				>
					<Icon data={faSearchMinus} />
				</Button>
			</div>
			<Skeleton loading={!jobs} layout={[[6], 1, [6], 1, [6], 1, [6], 1, [6]]} />
			{#if jobs}
				<div class="space-y-0">
					{#each jobs as job}
						<JobDetail {job} />
						<div class="line w-20 h-4" />
					{/each}
				</div>
			{/if}
		</div>
		{#if error}
			<div>
				{JSON.stringify(error)}
			</div>
		{/if}
	</div>

	<div class="text-center pb-6">
		{#if jobs && jobs.length >= jobsPerPage && showOlderJobs}
			<button class=" mt-4 text-blue-500 text-center text-sm" on:click={loadOlderJobs}>
				Load older jobs
			</button>
		{/if}
	</div>
</CenteredPage>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #999 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
