<script lang="ts">
	import { displayDate } from '$lib/utils'
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import Badge from './common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import DurationMs from './DurationMs.svelte'
	import { Calendar, CheckCircle2, Circle, Clock, Hourglass, Play, XCircle } from 'lucide-svelte'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'

	const SMALL_ICON_SIZE = 12

	export let job: QueuedJob | CompletedJob | undefined
</script>

{#if job && 'success' in job && job.success}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="green" icon={{ icon: CheckCircle2, position: 'left' }}>
			Success {job.is_skipped ? '(Skipped)' : ''}
		</Badge>
		<DurationMs
			duration_ms={job.duration_ms}
			self_wait_time_ms={job?.self_wait_time_ms}
			aggregate_wait_time_ms={job?.aggregate_wait_time_ms}
		/>
	</div>
{:else if job && 'success' in job}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="red" icon={{ icon: XCircle, position: 'left' }}>Failed</Badge>
		<DurationMs
			duration_ms={job.duration_ms}
			self_wait_time_ms={job?.self_wait_time_ms}
			aggregate_wait_time_ms={job?.aggregate_wait_time_ms}
		/>
	</div>
{:else if job && 'running' in job && job.running && job.suspend}
	<div>
		<Badge large color="violet" icon={{ icon: Hourglass, position: 'left' }}>
			Suspended
			{#if job.flow_status}
				({(job.flow_status?.step ?? 0) + 1} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
	</div>
{:else if job && 'running' in job && job.running}
	<div>
		<Badge large color="yellow" icon={{ icon: Play, position: 'left' }}>
			Running
			{#if job.flow_status}
				({(job.flow_status?.step ?? 0) + 1} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
	</div>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<div>
		<Badge color="blue" icon={{ icon: Calendar, position: 'left' }}>
			Scheduled for {displayDate(job.scheduled_for)}
		</Badge>
	</div>
{:else if job && 'running' in job}
	<div class="flex flex-row gap-1 items-center">
		<Badge color="orange" icon={{ icon: Clock, position: 'left' }}>Queued</Badge>
		<NoWorkerWithTagWarning tag={job.tag} />
	</div>
{:else}
	<Circle size={SMALL_ICON_SIZE} class="text-gray-200" />
{/if}
