<script lang="ts">
	import { ConcurrencyGroupsService, type Job, type WorkflowStatus } from '../../gen'
	import JobLoader from '../JobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Skeleton, Tab, Tabs } from '../common'
	import HighlightCode from '../HighlightCode.svelte'
	import FlowStatusViewer from '../FlowStatusViewer.svelte'
	import { workspaceStore } from '$lib/stores'
	import WorkflowTimeline from '../WorkflowTimeline.svelte'
	import { isFlowPreview, isScriptPreview, type StateStore } from '$lib/utils'
	import { setContext, untrack, createEventDispatcher } from 'svelte'
	import { LoaderCircle } from 'lucide-svelte'
	import FlowAssetsHandler, { initFlowGraphAssetsCtx } from '../flows/FlowAssetsHandler.svelte'
	import JobAssetsViewer from '../assets/JobAssetsViewer.svelte'
	import JobDetailHeader from './JobDetailHeader.svelte'
	import FlowExecutionStatus from './FlowExecutionStatus.svelte'

	interface Props {
		id: string
		workspace: string | undefined
	}

	let { id, workspace }: Props = $props()

	const dispatch = createEventDispatcher()

	let job: (Job & { result_stream?: string }) | undefined = $state(undefined)

	let currentJob: Job | undefined = $state(undefined)
	let loadError: string | undefined = $state(undefined)
	let isLoadingJobDetails: boolean = $state(false)

	let lastJobId: string | undefined = $state(undefined)
	let concurrencyKey: string | undefined = $state(undefined)
	async function getConcurrencyKey(job: Job) {
		lastJobId = job.id
		concurrencyKey = await ConcurrencyGroupsService.getConcurrencyKey({ id: job.id })
	}

	let viewTab = $state('logs')

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => job?.raw_flow?.modules ?? [] })
	)

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}

	function handleFilterByConcurrencyKey(key: string) {
		dispatch('filterByConcurrencyKey', key)
	}

	function handleFilterByWorker(worker: string) {
		dispatch('filterByWorker', worker)
	}
	$effect(() => {
		if (currentJob?.id == id) {
			job = currentJob
		}
	})
	$effect(() => {
		if (id && jobLoader) {
			loadError = undefined
			isLoadingJobDetails = true
			untrack(() =>
				jobLoader?.watchJob(id, {
					done(x) {
						loadError = undefined
						isLoadingJobDetails = false
					},
					doneError({ error }) {
						// Only show error state if we couldn't load the job data
						// If the job failed but we have the job object, let it display normally
						if (!currentJob) {
							loadError = (error as any)?.body || error?.message || 'Failed to load job details'
						}
						isLoadingJobDetails = false
					},
					change() {
						isLoadingJobDetails = false
					}
				})
			)
		}
	})

	$effect(() => {
		job?.id && lastJobId !== job.id && untrack(() => job && getConcurrencyKey(job))
	})

	let jobLoader: JobLoader | undefined = $state(undefined)

	// Set all tabs content to the same height to prevent layout jumps
	let tabsHeight = $state({
		codeHeight: 0,
		logsHeight: 0,
		assetsHeight: 0
	})

	let minTabHeight = $derived(
		Math.max(tabsHeight.codeHeight, tabsHeight.logsHeight, tabsHeight.assetsHeight)
	)

	let jobIsLoading = $state(false)

	// Flow execution status state
	let suspendStatus: StateStore<Record<string, { job: Job; nb: number }>> = $state({ val: {} })
	let isOwner: boolean = $state(false)
</script>

<JobLoader
	workspaceOverride={workspace}
	bind:job={currentJob}
	bind:isLoading={jobIsLoading}
	bind:this={jobLoader}
/>

<div class="h-full">
	<div class="flex flex-col items-start pb-4 min-h-full">
		{#if isLoadingJobDetails}
			<div class="w-full flex-1 flex items-center justify-center">
				<div class="text-center">
					<LoaderCircle size={32} class="animate-spin text-primary mx-auto" />
					<div class="text-secondary text-sm mt-2">Loading job details...</div>
				</div>
			</div>
		{:else if loadError}
			<div class="w-full h-full flex items-center justify-center">
				<div class="text-center">
					<div class="text-red-500 text-lg font-semibold mb-2">Error loading job</div>
					<div class="text-secondary text-sm">{loadError}</div>
				</div>
			</div>
		{:else if job}
			{@const isFlow = job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)}
			<JobDetailHeader
				{job}
				compact
				{concurrencyKey}
				onFilterByConcurrencyKey={handleFilterByConcurrencyKey}
				onFilterByWorker={handleFilterByWorker}
			/>

			<!-- Workflow timeline -->
			{#if job?.workflow_as_code_status}
				<div class="py-2">
					<WorkflowTimeline
						flow_status={asWorkflowStatus(job.workflow_as_code_status)}
						flowDone={job.type == 'CompletedJob'}
					/>
				</div>
			{/if}
			<div class="w-full mt-2">
				{#if isFlow}
					<FlowExecutionStatus
						{job}
						workspaceId={job?.workspace_id}
						{isOwner}
						innerModules={job?.flow_status?.modules}
						{suspendStatus}
					/>
				{/if}
			</div>

			<!-- Job inputs -->
			<div class="w-full mt-4">
				<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
				<JobArgs
					id={job?.id}
					workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
					args={job?.args}
				/>
			</div>

			<!-- Job execution content -->
			<div class="w-full mt-6">
				{#if isFlow}
					<FlowStatusViewer
						jobId={job.id}
						workspaceId={job.workspace_id}
						initialJob={job}
						bind:isOwner
						wideResults
					/>
				{:else if job?.type === 'CompletedJob'}
					<!-- Result Section (moved outside tabs) -->
					<div class="w-full mb-6">
						<h3 class="text-xs font-semibold text-emphasis mb-1">Result</h3>
						<div class="border rounded-md bg-surface-tertiary p-4 overflow-auto max-h-[400px]">
							{#if job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined)}
								<DisplayResult
									workspaceId={job?.workspace_id}
									jobId={job?.id}
									result={job?.result}
									disableExpand
									language={job?.language}
								/>
							{:else}
								<div class="text-secondary">No output is available yet</div>
							{/if}
						</div>
					</div>

					<Tabs bind:selected={viewTab}>
						<Tab value="logs" label="Logs" />
						<Tab value="assets" label="Assets" />
						{#if isScriptPreview(job?.job_kind)}
							<Tab value="code" label="Code" />
						{/if}
					</Tabs>

					<Skeleton loading={!job} layout={[[5]]} />
					{#if job}
						<div class="flex flex-col rounded-md mt-2 overflow-auto">
							{#if viewTab == 'logs'}
								<div
									class="w-full"
									bind:clientHeight={tabsHeight.logsHeight}
									style="min-height: {minTabHeight}px"
								>
									<LogViewer
										jobId={job.id}
										duration={job?.['duration_ms']}
										mem={job?.['mem_peak']}
										isLoading={job?.['running'] == false}
										content={job?.logs}
										tag={job?.tag}
									/>
								</div>
							{:else if viewTab == 'assets'}
								<div
									class="w-full h-full"
									bind:clientHeight={tabsHeight.assetsHeight}
									style="min-height: {minTabHeight}px"
								>
									<JobAssetsViewer {job} />
								</div>
							{:else if viewTab == 'code'}
								<div
									class="text-xs"
									bind:clientHeight={tabsHeight.codeHeight}
									style="min-height: {minTabHeight}px"
								>
									{#if job && 'raw_code' in job && job.raw_code}
										<div class="text-xs">
											<HighlightCode lines language={job.language} code={job.raw_code} />
										</div>
									{:else if job}
										<span class="text-sm">No code available</span>
									{:else}
										<Skeleton layout={[[5]]} />
									{/if}
								</div>
							{:else}
								<div class="w-full p-4 text-secondary">Select a tab to view content</div>
							{/if}
						</div>
					{/if}
				{:else if job && `running` in job ? job.running : false}
					<div class="text-sm font-semibold text-primary mb-1"> Job is still running </div>
					<LogViewer
						jobId={job?.id}
						duration={job?.['duration_ms']}
						mem={job?.['mem_peak']}
						content={job?.logs}
						isLoading={job?.['running'] == false}
						tag={job?.tag}
					/>
				{/if}
			</div>
		{:else if jobIsLoading}
			<div class="mx-auto my-auto">
				<LoaderCircle size={20} class="animate-spin" />
			</div>
		{/if}
	</div>
</div>
<FlowAssetsHandler
	modules={job?.raw_flow?.modules ?? []}
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
