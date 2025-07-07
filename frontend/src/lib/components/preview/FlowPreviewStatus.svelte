<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import { base } from '$lib/base'

	import JobStatus from '../JobStatus.svelte'
	import { ExternalLinkIcon } from 'lucide-svelte'
	import { truncateRev } from '$lib/utils'

	export let job: QueuedJob | CompletedJob
	export let hideJobId: boolean = false
</script>

<div class="grid grid-cols-2 gap-4 mb-1 text-tertiary dark:text-gray-400">
	<JobStatus {job} />
	{#if job && !hideJobId}
		<div>
			<div class="text-primary whitespace-nowrap truncate text-sm">
				<span class="font-semibold mr-1">Flow:</span>
				<a
					rel="noreferrer"
					target="_blank"
					href="{base}/run/{job?.id}?workspace={job?.workspace_id}"
				>
					{truncateRev(job?.id, 8)}
					<ExternalLinkIcon size={14} class="inline mb-1 ml-1" />
				</a>
			</div>
		</div>
	{/if}
</div>
