<script lang="ts">
	import { base } from '$lib/base'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import type { Job } from '$lib/gen'

	let {
		job,
		title = 'Result'
	}: {
		job: (Job & { result?: any }) | undefined
		/** What produced it: the run whose cell is open, or the trial that has just finished. */
		title?: string
	} = $props()
</script>

<!-- Framed like any other job result, and with the way to the rest of the job in its header: the
     trajectory, the logs and the tool calls are what the run page renders, and a second copy of
     them here would be a worse version of a page that already exists. -->
<div class="rounded-md border border-light overflow-hidden">
	<div class="flex items-center gap-2 px-2 py-1 border-b border-light bg-surface-secondary">
		<span class="text-2xs font-semibold text-secondary truncate">{title}</span>
		<div class="grow"></div>
		{#if job}
			<a
				class="text-2xs text-secondary hover:underline inline-flex items-center gap-1 shrink-0"
				href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}
				target="_blank"
			>
				Open the run
				<ExternalLink size={12} />
			</a>
		{/if}
	</div>
	<div class="p-2">
		{#if job?.type === 'CompletedJob'}
			<DisplayResult
				workspaceId={job.workspace_id}
				jobId={job.id}
				result={job.result}
				disableExpand
			/>
		{:else if job}
			<span class="text-xs text-tertiary">Running…</span>
		{:else}
			<span class="text-xs text-tertiary">Run the case to see its output here.</span>
		{/if}
	</div>
</div>
