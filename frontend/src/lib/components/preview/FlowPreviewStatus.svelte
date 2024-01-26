<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'

	import JobStatus from '../JobStatus.svelte'
	export let job: QueuedJob | CompletedJob
</script>

<div class="grid grid-cols-2 gap-4 mb-1 text-tertiary dark:text-gray-400">
	<JobStatus {job} />
	{#if job}
		<div>
			<div class="text-primary whitespace-nowrap truncate text-sm">
				<span class="font-semibold mr-1">Flow's job Id:</span>
				<a rel="noreferrer" target="_blank" href="/run/{job?.id}?workspace={job?.workspace_id}">
					{job?.id}
				</a>
			</div>
		</div>
	{/if}
</div>
