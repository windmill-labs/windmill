<script lang="ts">
	import { displayDate, forLater, msToSec } from '$lib/utils'
	import {
		faCalendar,
		faCheck,
		faCircle,
		faClock,
		faHourglassHalf,
		faTimes
	} from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'

	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import Badge from './common/badge/Badge.svelte'

	const SMALL_ICON_SCALE = 0.7

	export let job: QueuedJob | CompletedJob | undefined
</script>

{#if job && 'success' in job && job.success}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="green">
			<Icon data={faCheck} scale={SMALL_ICON_SCALE} class="mr-2" />
			Success {job.is_skipped ? '(Skipped)' : ''}
		</Badge>

		<Badge large>
			<Icon data={faHourglassHalf} scale={SMALL_ICON_SCALE} class="mr-2" />
			Ran in {msToSec(job.duration_ms)} s
		</Badge>
	</div>
{:else if job && 'success' in job}
	<div class="flex flex-row flex-wrap gap-y-1 mb-1 gap-x-2">
		<Badge large color="red">
			<Icon data={faTimes} scale={SMALL_ICON_SCALE} class="mr-2" />
			Failed
		</Badge>
		<Badge large>
			<Icon data={faHourglassHalf} scale={SMALL_ICON_SCALE} class="mr-2" />
			Ran in {msToSec(job.duration_ms)}s
		</Badge>
	</div>
{:else if job && 'running' in job && job.running}
	<div>
		<Badge large color="yellow">
			<Icon data={faCircle} scale={SMALL_ICON_SCALE} class="mr-2" />
			Running {#if job.flow_status}({job.flow_status?.step ?? ''} of {job.raw_flow?.modules
					?.length ?? '?'}){/if}
		</Badge>
	</div>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<div>
		<Badge>
			<Icon data={faCalendar} scale={SMALL_ICON_SCALE} class="mr-2" />
			Scheduled for {displayDate(job.scheduled_for)}
		</Badge>
	</div>
{:else if job && 'running' in job}
	<div>
		<Badge>
			<Icon data={faClock} scale={SMALL_ICON_SCALE} class="mr-2" />
			Queued
		</Badge>
	</div>
{:else}
	<Icon class="text-gray-200" data={faCircle} scale={SMALL_ICON_SCALE} />
{/if}
