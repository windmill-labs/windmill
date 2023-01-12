<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'

	import JobStatus from '../JobStatus.svelte'
	export let job: QueuedJob | CompletedJob
</script>

<div class="grid grid-cols-2 gap-4 mb-1 text-gray-500 dark:text-gray-400">
	<JobStatus {job} />
	{#if job}
		<div
			><div class=" text-gray-900 whitespace-nowrap truncate ">
				<span class="font-bold">Job Id</span>
				<a rel="noreferrer" target="_blank" href="/run/{job?.id}?workspace={job?.workspace_id}">
					{job?.id}
				</a>
			</div>
		</div>
	{/if}
</div>
