<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import { base } from '$lib/base'

	import JobStatus from '../JobStatus.svelte'
	import { ExternalLinkIcon } from 'lucide-svelte'
	import type { FlowStatusViewerContext } from '../graph'
	import { getContext } from 'svelte'
	export let job: QueuedJob | CompletedJob

	let { hideJobId } = getContext<FlowStatusViewerContext>('FlowStatusViewer')
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
					{job?.id}
					<ExternalLinkIcon size={14} class="inline mb-1 ml-1" />
				</a>
			</div>
		</div>
	{/if}
</div>
