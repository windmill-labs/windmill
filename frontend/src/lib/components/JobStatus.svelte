<script lang="ts">
	import { displayDate, msToReadableTime } from '$lib/utils'
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import Badge from './common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import { Circle } from 'lucide-svelte'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import QueuePosition from './QueuePosition.svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'

	const SMALL_ICON_SIZE = 12

	interface Props {
		job: QueuedJob | CompletedJob | undefined
		large?: boolean
	}

	let { job, large = false }: Props = $props()
</script>

{#if job && 'success' in job && job.success}
	<Badge {large} color="green">
		Successfully ran in {msToReadableTime(job.duration_ms)}
		{job.is_skipped ? '(Skipped)' : ''}
		{#if job.self_wait_time_ms || job.aggregate_wait_time_ms}
			<WaitTimeWarning
				self_wait_time_ms={job.self_wait_time_ms}
				aggregate_wait_time_ms={job.aggregate_wait_time_ms}
				variant="alert"
			/>
		{/if}
	</Badge>
{:else if job && 'success' in job}
	<Badge {large} color="red">
		Failed after {msToReadableTime(job.duration_ms)}
		{#if job.self_wait_time_ms || job.aggregate_wait_time_ms}
			<WaitTimeWarning
				self_wait_time_ms={job.self_wait_time_ms}
				aggregate_wait_time_ms={job.aggregate_wait_time_ms}
				variant="alert"
			/>
		{/if}
	</Badge>
{:else if job && 'running' in job && job.running && job.suspend}
	<div>
		<Badge {large} color="violet">
			Suspended
			{#if job.flow_status}
				({(job.flow_status?.step ?? 0) + 1} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
	</div>
{:else if job && 'running' in job && job.running}
	<div>
		<Badge {large} color="yellow">
			Running
			{#if job.flow_status}
				({(job.flow_status?.step ?? 0) + 1} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
	</div>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<div>
		<Badge color="blue" {large}>
			Scheduled for {displayDate(job.scheduled_for)}
		</Badge>
	</div>
{:else if job && 'running' in job}
	<div class="flex flex-row gap-1 items-center">
		<Badge color="orange" {large}>Queued</Badge>
		<NoWorkerWithTagWarning tag={job.tag} />
		<QueuePosition jobId={job.id} workspaceId={job.workspace_id} minimal />
	</div>
{:else}
	<Circle size={SMALL_ICON_SIZE} class="text-gray-200" />
{/if}
