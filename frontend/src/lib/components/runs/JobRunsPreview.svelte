<script lang="ts">
	import { ConcurrencyGroupsService, type Job, type WorkflowStatus } from '../../gen'
	import JobLoader from '../JobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Skeleton, Tab, Tabs } from '../common'
	import HighlightCode from '../HighlightCode.svelte'
	import { forLater } from '$lib/forLater'
	import FlowProgressBar from '../flows/FlowProgressBar.svelte'
	import FlowStatusViewer from '../FlowStatusViewer.svelte'
	import { workspaceStore } from '$lib/stores'
	import WorkflowTimeline from '../WorkflowTimeline.svelte'
	import { isFlowPreview, isScriptPreview } from '$lib/utils'
	import { setContext, untrack } from 'svelte'
	import { Calendar, LoaderCircle } from 'lucide-svelte'
	import FlowAssetsHandler, { initFlowGraphAssetsCtx } from '../flows/FlowAssetsHandler.svelte'
	import JobAssetsViewer from '../assets/JobAssetsViewer.svelte'
	import JobDetailHeader from './JobDetailHeader.svelte'

	interface Props {
		id: string
		workspace: string | undefined
	}

	let { id, workspace }: Props = $props()

	let job: (Job & { result_stream?: string }) | undefined = $state(undefined)

	let result: any = $state()

	function onDone(job: Job) {
		result = job['result']
	}

	let currentJob: Job | undefined = $state(undefined)

	let lastJobId: string | undefined = $state(undefined)
	let concurrencyKey: string | undefined = $state(undefined)
	async function getConcurrencyKey(job: Job) {
		lastJobId = job.id
		concurrencyKey = await ConcurrencyGroupsService.getConcurrencyKey({ id: job.id })
	}

	let viewTab = $state('result')

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => job?.raw_flow?.modules ?? [] })
	)

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}
	$effect(() => {
		if (currentJob?.id == id) {
			job = currentJob
		}
	})
	$effect(() => {
		id &&
			jobLoader &&
			untrack(() =>
				jobLoader?.watchJob(id, {
					done(x) {
						onDone(x)
					}
				})
			)
	})

	$effect(() => {
		job?.id && lastJobId !== job.id && untrack(() => job && getConcurrencyKey(job))
	})

	let jobLoader: JobLoader | undefined = $state(undefined)

	// Set all tabs content to the same height to prevent layout jumps
	let tabsHeight = $state({
		codeHeight: 0,
		logsHeight: 0,
		assetsHeight: 0,
		resultHeight: 0
	})

	let minTabHeight = $derived(
		Math.max(
			tabsHeight.codeHeight,
			tabsHeight.logsHeight,
			tabsHeight.assetsHeight,
			tabsHeight.resultHeight
		)
	)

	let jobIsLoading = $state(false)
</script>

<JobLoader
	workspaceOverride={workspace}
	bind:job={currentJob}
	bind:isLoading={jobIsLoading}
	bind:this={jobLoader}
/>

<div class="h-full overflow-y-auto">
	<div class="flex flex-col gap-2 items-start p-4 pb-8 min-h-full">
		{#if job}
			<JobDetailHeader {job} compact {concurrencyKey} />

			<div class="w-full mt-6">
				<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
				<JobArgs
					id={job?.id}
					workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
					args={job?.args}
				/>
			</div>

			{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
				<div
					class="flex flex-row gap-4 items-center mb-1 w-full bg-surface-tertiary rounded-md p-4 border"
				>
					<Calendar size={16} class="text-accent" />
					<div class="flex flex-col gap-1">
						<span class="text-2xs font-normal text-secondary">Job is scheduled for</span>
						<span class="text-xs font-semibold text-emphasis"
							>{new Date(job?.['scheduled_for']).toLocaleString()}</span
						>
					</div>
				</div>
			{/if}

			<div class="w-full rounded-md min-h-full">
				{#if job?.workflow_as_code_status}
					<WorkflowTimeline
						flow_status={asWorkflowStatus(job.workflow_as_code_status)}
						flowDone={job.type == 'CompletedJob'}
					/>
				{/if}

				{#if job?.type === 'CompletedJob'}
					{#if job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)}
						<div class="w-full mt-8 mb-20">
							<FlowStatusViewer
								jobId={job.id}
								workspaceId={job.workspace_id}
								wideResults
								initialJob={job}
							></FlowStatusViewer>
						</div>
					{:else}
						<Tabs bind:selected={viewTab}>
							<Tab value="result" label="Results" />
							<Tab value="logs" label="Logs" />
							<Tab value="assets" label="Assets" />
							{#if isScriptPreview(job?.job_kind)}
								<Tab value="code" label="Code" />
							{/if}
						</Tabs>

						<Skeleton loading={!job} layout={[[5]]} />
						{#if job}
							<div class="flex flex-col border rounded-md p-2 mt-2 overflow-auto">
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
								{:else if job !== undefined && (job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined))}
									<div
										class="w-full"
										bind:clientHeight={tabsHeight.resultHeight}
										style="min-height: {minTabHeight}px"
									>
										<DisplayResult
											workspaceId={job?.workspace_id}
											jobId={job?.id}
											{result}
											disableExpand
											language={job?.language}
										/>
									</div>
								{:else if job}
									No output is available yet
								{/if}
							</div>
						{/if}
					{/if}
				{:else if job && `running` in job ? job.running : false}
					{#if job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)}
						<div class="flex flex-col gap-2 w-full">
							<FlowProgressBar {job} class="py-4" />
							<FlowStatusViewer jobId={job.id} workspaceId={job.workspace_id} initialJob={job} />
						</div>
					{:else}
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
