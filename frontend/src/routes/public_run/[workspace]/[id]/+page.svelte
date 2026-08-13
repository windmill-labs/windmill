<script lang="ts">
	import { page } from '$app/state'
	import { onDestroy, untrack } from 'svelte'
	import { Globe } from 'lucide-svelte'
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { setViewToken } from '$lib/viewToken'
	import { setLicense } from '$lib/enterpriseUtils'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { displayDate, isNotFlow, truncateRev } from '$lib/utils'
	import { Alert, Badge, Skeleton } from '$lib/components/common'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'

	// SvelteKit reuses this component across /public_run/* navigations, so every input
	// derived from the URL has to stay reactive — a captured-once value would leave the
	// previous run (and its token) live while the address bar shows another link.
	let workspace = $derived(page.params.workspace ?? '')
	let jobId = $derived(page.params.id ?? '')
	let viewToken = $derived(page.url.searchParams.get('view_token') ?? undefined)
	// Identity of what is on screen. The token is part of it: the same run reached with a
	// different credential is a different view, and a completed job makes no further
	// request that would reject a now-invalid one.
	let viewKey = $derived(`${workspace}|${jobId}|${viewToken ?? ''}`)

	// The public share token is the page's only credential: install it before JobLoader
	// mounts so every read it fires (job, args, logs, SSE, flow steps) carries it. Set
	// eagerly at init, then kept in sync for client-side navigation.
	setViewToken(viewToken)
	$effect(() => {
		setViewToken(viewToken)
	})
	onDestroy(() => setViewToken(undefined))

	$effect(() => {
		$workspaceStore = workspace
	})

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	document.documentElement.classList.toggle('dark', darkMode === 'dark')
	applyDarkModeVariant()

	setLicense()

	let job: (Job & { result?: any; result_stream?: string }) | undefined = $state()
	let notfound = $state(false)
	let loadError: { status?: number; message?: string } | undefined = $state(undefined)
	let jobLoader: JobLoader | undefined = $state(undefined)

	let isFlow = $derived(job != undefined && !isNotFlow(job?.job_kind))

	// Clear the previous run first: `isFlow` then falls back to false, which remounts the
	// JobLoader a flow run had unmounted, so the effect below has a loader to watch with.
	let resetForViewKey: string | undefined = undefined
	$effect(() => {
		const key = viewKey
		untrack(() => {
			if (resetForViewKey === key) return
			resetForViewKey = key
			job = undefined
			notfound = false
			loadError = undefined
		})
	})

	// `watchJob` writes `job`, so it must not run tracked — reading its own output back
	// would re-enter this effect on every poll.
	let watchedViewKey: string | undefined = undefined
	$effect(() => {
		const key = viewKey
		const id = jobId
		const loader = jobLoader
		untrack(() => {
			if (!loader || !id || watchedViewKey === key) return
			watchedViewKey = key
			loader.watchJob(id)
		})
	})
</script>

<svelte:head>
	<title>Run {truncateRev(jobId, 8)} | Windmill</title>
	<!-- The URL *is* the credential: keep the link out of search indexes, and out of the
	     Referer header of anything the rendered result or logs may link to. -->
	<meta name="robots" content="noindex, nofollow" />
	<meta name="referrer" content="no-referrer" />
</svelte:head>

<!-- Flow jobs are watched by FlowStatusViewer's own loader, exactly like the run page. -->
{#if !isFlow}
	<JobLoader
		bind:this={jobLoader}
		bind:job
		bind:notfound
		bind:loadError
		workspaceOverride={workspace}
	/>
{/if}

<div class="min-h-screen bg-surface">
	<div class="border-b bg-surface-secondary">
		<div class="max-w-7xl mx-auto w-full px-4 py-3 flex items-center gap-3">
			<WindmillIcon height="20px" width="20px" />
			<span class="text-sm font-semibold text-primary">Run {truncateRev(jobId, 8)}</span>
			<Badge color="blue" small>
				<span class="flex items-center gap-1"><Globe size={12} /> Public read-only view</span>
			</Badge>
		</div>
	</div>

	<div class="max-w-7xl mx-auto w-full px-4 py-6 flex flex-col gap-6">
		{#if loadError || notfound}
			<Alert type="error" title="This run is not available">
				{#if loadError?.status === 403 || loadError?.status === 400}
					This link is not valid for this run, or public sharing of it was never enabled.
				{:else}
					Run {jobId} was not found in {workspace}.
				{/if}
			</Alert>
		{:else if !job}
			<Skeleton layout={[[3], 1, [8]]} />
		{:else}
			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-2">
					<JobStatus {job} />
					{#if job.script_path}
						<Badge color="gray">{job.script_path}</Badge>
					{/if}
					{#if job.language}
						<Badge color="gray">{job.language}</Badge>
					{/if}
				</div>
				<div class="text-xs text-secondary">
					Started {job.started_at ? displayDate(job.started_at) : 'not yet'}
				</div>
			</div>

			<div>
				<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
				<JobArgs workspace={job.workspace_id ?? workspace} id={job.id} args={job.args} />
			</div>

			{#if isFlow}
				<FlowStatusViewer
					jobId={job.id}
					initialJob={job}
					workspaceId={workspace}
					onJobsLoaded={({ job: newJob }) => (job = newJob)}
					onDone={({ job: newJob }) => (job = newJob)}
				/>
			{:else}
				<div>
					<div class="text-xs text-emphasis font-semibold mb-1">Result</div>
					<div class="border rounded-md bg-surface-tertiary p-4 overflow-auto max-h-screen">
						{#if job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined)}
							<DisplayResult
								workspaceId={job.workspace_id}
								result_stream={job.result_stream}
								jobId={job.id}
								result={job.result}
								language={job.language}
								isTest={false}
							/>
						{:else}
							<div class="text-secondary text-sm">No output is available yet</div>
						{/if}
					</div>
				</div>

				<div>
					<div class="text-xs text-emphasis font-semibold mb-1">Logs</div>
					<div class="border rounded-md bg-surface-secondary p-2 overflow-auto min-h-[400px]">
						<LogViewer
							jobId={job.id}
							duration={job?.['duration_ms']}
							mem={job?.['mem_peak']}
							isLoading={job?.['running'] == false}
							content={job?.logs}
							tag={job?.tag}
						/>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>
