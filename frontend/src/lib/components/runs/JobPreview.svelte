<script lang="ts">
	import { onDestroy } from 'svelte'
	import { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { forLater, msToSec } from '$lib/utils'
	import { Badge, Button, Skeleton, Tab, Tabs } from '../common'
	import { onMount } from 'svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import { goto } from '$app/navigation'

	export let id: string
	let job: Job | undefined = undefined
	let timeout: NodeJS.Timeout | undefined
	let watchJob: (id: string) => Promise<void>
	let result: any

	function onDone(event: { detail: Job }) {
		job = event.detail
		result = job['result']
	}

	onMount(() => {
		watchJob(id)
	})

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})

	let viewTab = 'result'
</script>

<svelte:window on:keydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()} />
<TestJobLoader bind:job bind:watchJob on:done={onDone} />

<div class="p-4 flex flex-col gap-2 items-start">
	<div class="flex gap-2">
		<Badge color="blue">
			{#if job && 'duration_ms' in job && job.duration_ms != undefined}
				Ran in ({msToSec(job.duration_ms)}s)
			{/if}
		</Badge>
		{#if job?.['mem_peak']}
			<Badge color="blue">
				Mem: {job?.['mem_peak'] ? `${(job['mem_peak'] / 1024).toPrecision(4)}MB` : 'N/A'}
			</Badge>
		{/if}
	</div>
	<Button
		class="flex flex-row gap-1 items-center"
		on:click={() => {
			goto(`/run/${job?.id}`)
		}}
	>
		<span class="font-semibold text-xs leading-6">ID:</span>
		<span class="text-xs">{job?.id}</span>
	</Button>

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
				{#if job?.job_kind == 'dependencies'}
					<Tab size="xs" value="code">Code</Tab>
				{:else if job?.job_kind == 'preview'}
					<Tab size="xs" value="code">Code</Tab>
				{/if}
			</Tabs>

			<Skeleton loading={!job} layout={[[5]]} />
			{#if job}
				<div class="flex flex-row border rounded-md p-2 mt-2 max-h-1/2 overflow-auto">
					{#if viewTab == 'logs'}
						<div class="w-full">
							<LogViewer
								jobId={job.id}
								duration={job?.['duration_ms']}
								mem={job?.['mem_peak']}
								isLoading={!(job && 'logs' in job && job.logs)}
								content={job?.logs}
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
						<DisplayResult workspaceId={job?.workspace_id} jobId={job?.id} {result} disableExpand />
					{:else if job}
						No output is available yet
					{/if}
				</div>
			{/if}
		{:else if job && `running` in job ? job.running : false}
			<div class="text-sm font-semibold text-tertiary mb-1"> Job is still running </div>
			<LogViewer
				jobId={job?.id}
				duration={job?.['duration_ms']}
				mem={job?.['mem_peak']}
				content={job?.logs}
				isLoading
			/>
		{/if}
	</div>
</div>
