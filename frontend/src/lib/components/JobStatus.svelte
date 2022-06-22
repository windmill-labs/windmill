<script lang="ts">
	import {
		faCalendar,
		faCircle,
		faClock,
		faHourglassHalf,
		faTimes
	} from '@fortawesome/free-solid-svg-icons'
	import { displayDate, forLater } from '$lib/utils'

	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'

	import type { CompletedJob, QueuedJob } from '$lib/gen'

	const SMALL_ICON_SCALE = 0.7

	export let job: QueuedJob | CompletedJob | undefined
</script>

{#if job && 'success' in job && job.success}
	<Icon class="text-green-600" data={check} scale={SMALL_ICON_SCALE} />
	<span class="mx-2">Succeeded</span>
	<div>
		<Icon class="text-gray-700" data={faHourglassHalf} scale={SMALL_ICON_SCALE} /><span class="mx-2"
			>Job ran in {job.duration}
			s</span
		>
	</div>
{:else if job && 'success' in job}
	<Icon class="text-red-700" data={faTimes} scale={SMALL_ICON_SCALE} />
	<span class="mx-2">Failed</span>
	<div>
		<Icon class="text-gray-700" data={faHourglassHalf} scale={SMALL_ICON_SCALE} /><span class="mx-2"
			>Job ran in {job.duration}
			s</span
		>
	</div>
{:else if job && 'running' in job && job.running}
	<Icon class="text-yellow-500" data={faCircle} scale={SMALL_ICON_SCALE} />
	<span class="mx-2">Running</span>
{:else if job && 'running' in job && 'scheduled_for' in job && job.scheduled_for && forLater(job.scheduled_for)}
	<Icon class="text-gray-700" data={faCalendar} scale={SMALL_ICON_SCALE} />
	<span class="mx-2">Scheduled for {displayDate(job.scheduled_for)}</span>
{:else if job && 'running' in job}
	<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} />
	<span class="mx-2">Queued</span>
{:else}
	<Icon class="text-gray-200" data={faCircle} scale={SMALL_ICON_SCALE} />
{/if}
