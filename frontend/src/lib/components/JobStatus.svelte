<script lang="ts">
	import { displayDate } from '$lib/utils'
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import Badge from './common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import DurationMs from './DurationMs.svelte'
	import { Calendar, CheckCircle2, Circle, Clock, XCircle } from 'lucide-svelte'

	const SMALL_ICON_SIZE = 12

	export let job: QueuedJob | CompletedJob | undefined
</script>

{#if job && 'success' in job && job.success}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="green">
			<CheckCircle2 size={SMALL_ICON_SIZE} class="mr-2" />
			Success {job.is_skipped ? '(Skipped)' : ''}
		</Badge>
		<DurationMs duration_ms={job.duration_ms} />
	</div>
{:else if job && 'success' in job}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="red">
			<XCircle size={SMALL_ICON_SIZE} class="mr-2" />
			Failed
		</Badge>
		<DurationMs duration_ms={job.duration_ms} />
	</div>
{:else if job && 'running' in job && job.running}
	<div>
		<Badge large color="yellow">
			<Clock size={SMALL_ICON_SIZE} class="mr-2" />
			Running
			{#if job.flow_status}
				({job.flow_status?.step + 1 ?? ''} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
	</div>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<div>
		<Badge color="blue">
			<Calendar size={SMALL_ICON_SIZE} class="mr-2" />
			Scheduled for {displayDate(job.scheduled_for)}
		</Badge>
	</div>
{:else if job && 'running' in job}
	<div>
		<Badge>
			<Clock size={SMALL_ICON_SIZE} class="mr-2" />
			Queued
		</Badge>
	</div>
{:else}
	<Circle size={SMALL_ICON_SIZE} class="text-gray-200" />
{/if}
