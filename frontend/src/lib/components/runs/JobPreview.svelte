<script lang="ts">
	import { onDestroy } from 'svelte'
	import { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import LogViewer from '../LogViewer.svelte'
	import { forLater, msToSec } from '$lib/utils'
	import { Badge } from '../common'
	import { onMount } from 'svelte'

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
</script>

<svelte:window on:keydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()} />
<TestJobLoader bind:job bind:watchJob on:done={onDone} />

<div class="p-4 flex flex-col gap-2 border-t items-start">
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
	<span class="font-semibold text-xs leading-6">ID</span>
	<span class="text-xs">{job?.id}</span>

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

	<div class="border p-2 w-full rounded-md">
		{#if job?.type === Job.type.COMPLETED_JOB}
			<DisplayResult workspaceId={job?.workspace_id} jobId={job?.id} {result} disableExpand />
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
