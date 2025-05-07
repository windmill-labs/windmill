<script lang="ts">
	import { base } from '$lib/base'
	import { ConcurrencyGroupsService, type Job, type WorkflowStatus } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Badge, Button, Skeleton, Tab, Tabs } from '../common'
	import HighlightCode from '../HighlightCode.svelte'
	import { forLater } from '$lib/forLater'
	import FlowProgressBar from '../flows/FlowProgressBar.svelte'
	import FlowStatusViewer from '../FlowStatusViewer.svelte'
	import DurationMs from '../DurationMs.svelte'
	import { workspaceStore } from '$lib/stores'
	import WorkflowTimeline from '../WorkflowTimeline.svelte'
	import Popover from '../Popover.svelte'
	import { isFlowPreview, isScriptPreview, truncateRev } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { ListFilter } from 'lucide-svelte'

	export let id: string
	export let blankLink = false
	export let workspace: string | undefined

	let job: Job | undefined = undefined
	let watchJob: ((id: string) => Promise<void>) | undefined = undefined
	let getLogs: (() => Promise<void>) | undefined = undefined

	let result: any

	function onDone(event: { detail: Job }) {
		job = event.detail
		result = job['result']
	}

	let currentJob: Job | undefined = undefined

	$: if (currentJob?.id == id) {
		job = currentJob
	}

	$: id && watchJob && watchJob(id)

	$: job?.logs == undefined && job && viewTab == 'logs' && getLogs?.()

	let lastJobId: string | undefined = undefined
	let concurrencyKey: string | undefined = undefined
	$: job?.id && lastJobId !== job.id && getConcurrencyKey(job)
	async function getConcurrencyKey(job: Job) {
		lastJobId = job.id
		concurrencyKey = await ConcurrencyGroupsService.getConcurrencyKey({ id: job.id })
	}

	let viewTab = 'result'

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}
	const dispatch = createEventDispatcher()
</script>

<TestJobLoader
	lazyLogs
	workspaceOverride={workspace}
	bind:job={currentJob}
	bind:getLogs
	bind:watchJob
	on:done={onDone}
/>

<div class="p-4 flex flex-col gap-2 items-start h-full">
	{#if job}
		<div class="flex gap-2 flex-wrap">
			{#if job?.['priority']}
				<Badge color="red">
					priority: {job?.['priority']}
				</Badge>
			{/if}
			{#if job && 'duration_ms' in job && job.duration_ms != undefined}
				<DurationMs
					duration_ms={job.duration_ms}
					self_wait_time_ms={job?.self_wait_time_ms}
					aggregate_wait_time_ms={job?.aggregate_wait_time_ms}
				/>
			{/if}
			{#if job?.['mem_peak']}
				<Badge large>
					Mem: {job?.['mem_peak'] ? `${(job['mem_peak'] / 1024).toPrecision(4)}MB` : 'N/A'}
				</Badge>
			{/if}
			{#if workspace && $workspaceStore != workspace}
				<Badge large>
					Workspace: {workspace}
				</Badge>
			{/if}
			{#if job.tag}
				<Badge large>
					Tag: {job.tag}
				</Badge>
			{/if}
			{#if job?.['labels'] && Array.isArray(job?.['labels']) && job?.['labels'].length > 0}
				{#each job?.['labels'] as label}
					<Badge baseClass="text-2xs">Label: {label}</Badge>
				{/each}
			{/if}
			{#if concurrencyKey}
				<Popover notClickable>
					<svelte:fragment slot="text">
						This job has concurrency limits enabled with the key:
						<Button
							class="inline-text"
							size="xs2"
							color="light"
							on:click={() => {
								dispatch('filterByConcurrencyKey', concurrencyKey)
							}}
						>
							{concurrencyKey}
							<ListFilter class="inline-block" size={10} />
						</Button>
					</svelte:fragment>
					<Badge large>Concurrency: {truncateRev(concurrencyKey, 20)}</Badge>
				</Popover>
			{/if}
			{#if job?.worker}
				<Popover notClickable>
					<svelte:fragment slot="text">
						This job was run on worker:
						<Button
							class="inline-text"
							size="xs2"
							color="light"
							on:click={() => {
								dispatch('filterByWorker', job?.worker)
							}}
						>
							{job.worker}
							<ListFilter class="inline-block" size={10} />
						</Button>
					</svelte:fragment>
					<Badge large>Worker: {truncateRev(job.worker, 20)}</Badge>
				</Popover>
			{/if}
		</div>
		<a
			href="{base}/run/{job?.id}?workspace={job?.workspace_id}"
			class="flex flex-row gap-1 items-center"
			target={blankLink ? '_blank' : undefined}
		>
			<span class="font-semibold text-sm leading-6">ID:</span>
			<span class="text-sm">{job?.id ?? ''}</span>
		</a>

		<span class="font-semibold text-xs leading-6">Arguments</span>

		<div class="w-full">
			<JobArgs
				id={job?.id}
				workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
				args={job?.args}
			/>
		</div>

		{#if job?.type === 'CompletedJob'}
			<span class="font-semibold text-xs leading-6">Results</span>
		{/if}

		{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
			<div class="text-sm font-semibold text-tertiary mb-1">
				<div>Job is scheduled for</div>
				<div>{new Date(job?.['scheduled_for']).toLocaleString()}</div>
			</div>
		{/if}

		<div class=" w-full rounded-md min-h-full">
			{#if job?.is_flow_step == false && job?.flow_status && (isScriptPreview(job?.job_kind) || job?.job_kind == 'script') && !(typeof job.flow_status == 'object' && '_metadata' in job.flow_status)}
				<WorkflowTimeline
					flow_status={asWorkflowStatus(job.flow_status)}
					flowDone={job.type == 'CompletedJob'}
				/>
			{/if}

			{#if job?.type === 'CompletedJob'}
				<Tabs bind:selected={viewTab}>
					<Tab size="xs" value="result">Result</Tab>
					<Tab size="xs" value="logs">Logs</Tab>
					{#if isScriptPreview(job?.job_kind)}
						<Tab size="xs" value="code">Code</Tab>
					{/if}
				</Tabs>

				<Skeleton loading={!job} layout={[[5]]} />
				{#if job}
					{#if viewTab == 'result' && (job?.job_kind == 'flow' || isFlowPreview(job?.job_kind))}
						<div class="flex flex-col gap-2">
							<div class="w-full mt-10 mb-20">
								<FlowStatusViewer jobId={job.id} workspaceId={job.workspace_id} />
							</div>
						</div>
					{:else}
						<div class="flex flex-col border rounded-md p-2 mt-2 h-full overflow-auto">
							{#if viewTab == 'logs'}
								<div class="w-full">
									<LogViewer
										jobId={job.id}
										duration={job?.['duration_ms']}
										mem={job?.['mem_peak']}
										isLoading={job?.['running'] == false}
										content={job?.logs}
										tag={job?.tag}
									/>
								</div>
							{:else if viewTab == 'code'}
								{#if job && 'raw_code' in job && job.raw_code}
									<div class="text-xs">
										<HighlightCode lines language={job.language} code={job.raw_code} />
									</div>
								{:else if job}
									No code is available
								{:else}
									<Skeleton layout={[[5]]} />
								{/if}
							{:else if job !== undefined && 'result' in job && job.result !== undefined}
								<DisplayResult
									workspaceId={job?.workspace_id}
									jobId={job?.id}
									{result}
									disableExpand
									language={job?.language}
								/>
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
						<FlowStatusViewer jobId={job.id} workspaceId={job.workspace_id} />
					</div>
				{:else}
					<div class="text-sm font-semibold text-tertiary mb-1"> Job is still running </div>
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
	{/if}
</div>
