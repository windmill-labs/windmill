<script lang="ts">
	import { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { Badge, Skeleton, Tab, Tabs } from '../common'
	import HighlightCode from '../HighlightCode.svelte'
	import { forLater } from '$lib/forLater'
	import FlowProgressBar from '../flows/FlowProgressBar.svelte'
	import FlowStatusViewer from '../FlowStatusViewer.svelte'
	import DurationMs from '../DurationMs.svelte'

	export let id: string
	export let blankLink = false

	let job: Job | undefined = undefined
	let watchJob: (id: string) => Promise<void>
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

	let viewTab = 'result'
</script>

<TestJobLoader bind:job={currentJob} bind:watchJob on:done={onDone} />

<div class="p-4 flex flex-col gap-2 items-start">
	{#if job}
		<div class="flex gap-2">
			{#if job?.['priority']}
				<Badge color="red">
					priority: {job?.['priority']}
				</Badge>
			{/if}
			{#if job && 'duration_ms' in job && job.duration_ms != undefined}
				<DurationMs duration_ms={job.duration_ms} />
			{/if}
			{#if job?.['mem_peak']}
				<Badge large>
					Mem: {job?.['mem_peak'] ? `${(job['mem_peak'] / 1024).toPrecision(4)}MB` : 'N/A'}
				</Badge>
			{/if}
		</div>
		<a
			href="/run/{job?.id}?workspace={job?.workspace_id}"
			class="flex flex-row gap-1 items-center"
			target={blankLink ? '_blank' : undefined}
		>
			<span class="font-semibold text-sm leading-6">ID:</span>
			<span class="text-sm">{job?.id ?? ''}</span>
		</a>

		<span class="font-semibold text-xs leading-6">Arguments</span>

		<div class="w-full">
			<JobArgs args={job?.args} />
		</div>

		<span class="font-semibold text-xs leading-6">Results</span>

		{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
			<div class="text-sm font-semibold text-tertiary mb-1">
				<div>Job is scheduled for</div>
				<div>{new Date(job?.['scheduled_for']).toLocaleString()}</div>
			</div>
		{/if}

		<div class=" w-full rounded-md overflow-auto">
			{#if job?.type === Job.type.COMPLETED_JOB}
				<Tabs bind:selected={viewTab}>
					<Tab size="xs" value="result">Result</Tab>
					<Tab size="xs" value="logs">Logs</Tab>
					{#if job?.job_kind == 'preview'}
						<Tab size="xs" value="code">Code</Tab>
					{/if}
				</Tabs>

				<Skeleton loading={!job} layout={[[5]]} />
				{#if job}
					{#if viewTab == 'result' && (job?.job_kind == 'flow' || job?.job_kind == 'flowpreview')}
						<div class="flex flex-col gap-2">
							<div class="w-full mt-10 mb-20">
								<FlowStatusViewer jobId={job.id} workspaceId={job.workspace_id} />
							</div>
						</div>
					{:else}
						<div class="flex flex-row border rounded-md p-2 mt-2 max-h-1/2 overflow-auto">
							{#if viewTab == 'logs'}
								<div class="w-full">
									<LogViewer
										jobId={job.id}
										duration={job?.['duration_ms']}
										mem={job?.['mem_peak']}
										isLoading={!(job && 'logs' in job && job.logs)}
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
								/>
							{:else if job}
								No output is available yet
							{/if}
						</div>
					{/if}
				{/if}
			{:else if job && `running` in job ? job.running : false}
				{#if job?.job_kind == 'flow' || job?.job_kind == 'flowpreview'}
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
						isLoading
						tag={job?.tag}
					/>
				{/if}
			{/if}
		</div>
	{/if}
</div>
