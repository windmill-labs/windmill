<script lang="ts">
	import { displayDate } from '$lib/utils'
	import type { QueuedJob, CompletedJob } from '$lib/gen'
	import Badge from './common/badge/Badge.svelte'
	import { forLater } from '$lib/forLater'
	import DurationMs from './DurationMs.svelte'
	import { Calendar, CheckCircle2, Circle, Clock, XCircle, XOctagon } from 'lucide-svelte'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import Tooltip from './Tooltip.svelte'

	const SMALL_ICON_SIZE = 12

	export let job: QueuedJob | CompletedJob | undefined

	function isJobHanging(job: QueuedJob): boolean {
		const lastUpdateTs = job.flow_last_progress_ts
		if (lastUpdateTs === undefined) {
			return false
		}
		if (Date.parse(lastUpdateTs) < new Date().getTime() - 1000 * 60) {
			return true
		}
		return false
	}
</script>

{#if job && 'success' in job && job.success}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="green" icon={{ icon: CheckCircle2, position: 'left' }}>
			Success {job.is_skipped ? '(Skipped)' : ''}
		</Badge>
		<DurationMs
			flow={job.job_kind == 'flow' || job?.job_kind == 'flowpreview'}
			duration_ms={job.duration_ms}
		/>
	</div>
{:else if job && 'success' in job}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="red" icon={{ icon: XCircle, position: 'left' }}>Failed</Badge>
		<DurationMs
			flow={job.job_kind == 'flow' || job?.job_kind == 'flowpreview'}
			duration_ms={job.duration_ms}
		/>
	</div>
{:else if job && 'running' in job && job.running}
	<div>
		<Badge large color="yellow" icon={{ icon: Clock, position: 'left' }}>
			Running
			{#if job.flow_status}
				({job.flow_status?.step + 1 ?? ''} of {job.raw_flow?.modules?.length ?? '?'})
			{/if}
		</Badge>
		{#if isJobHanging(job)}
			<Badge large color="red" icon={{ icon: XOctagon, position: 'left' }}>
				Hanging
				<Tooltip>
					The flow job is hanging in between 2 steps and not making any progress. Please
					"force-cancel" the job by clicking the above button and restart it manually.
				</Tooltip>
			</Badge>
		{/if}
	</div>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<div>
		<Badge color="blue" icon={{ icon: Calendar, position: 'left' }}>
			Scheduled for {displayDate(job.scheduled_for)}
		</Badge>
	</div>
{:else if job && 'running' in job}
	<div class="flex flex-row gap-1 items-center">
		<Badge icon={{ icon: Clock, position: 'left' }}>Queued</Badge>
		<NoWorkerWithTagWarning tag={job.tag} />
	</div>
{:else}
	<Circle size={SMALL_ICON_SIZE} class="text-gray-200" />
{/if}
