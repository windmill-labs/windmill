<script lang="ts">
	import type { CompletedJob, QueuedJob } from '$lib/gen'
	import { base } from '$lib/base'

	import JobStatus from '../JobStatus.svelte'
	import { ExternalLinkIcon } from 'lucide-svelte'
	import { truncateRev } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		job: QueuedJob | CompletedJob
		hideJobId?: boolean
		extra?: import('svelte').Snippet
	}

	let { job, hideJobId = false, extra }: Props = $props()
</script>

<div
	class={twMerge(
		'grid grid-cols-2 gap-4 mb-1 text-primary ',
		extra && job && !hideJobId ? 'grid-cols-3' : 'grid-cols-2'
	)}
>
	<JobStatus {job} />
	{#if job && !hideJobId}
		<div>
			<div class="text-primary whitespace-nowrap truncate text-xs">
				{#if ['flow', 'flowpreview', 'flownode'].includes(job.job_kind)}
					<span class="font-semibold text-emphasis text-xs mr-1">Flow:</span>
				{/if}
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
	{@render extra?.()}
</div>
