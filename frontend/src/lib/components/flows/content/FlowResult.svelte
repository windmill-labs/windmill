<script lang="ts">
	import FlowExecutionStatus from '$lib/components/runs/FlowExecutionStatus.svelte'
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowCard from '../common/FlowCard.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { StateStore } from '$lib/utils'
	import JobDetailHeader from '$lib/components/runs/JobDetailHeader.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'

	interface Props {
		job?: Job
		isOwner?: boolean
		suspendStatus?: StateStore<Record<string, { job: Job; nb: number }>>
		noEditor: boolean
		onOpenDetails?: () => void
	}

	let { job, isOwner, suspendStatus, noEditor, onOpenDetails }: Props = $props()
</script>

<FlowCard {noEditor} title="Flow result">
	<div class="px-4 py-2">
		{#if job}
			<!-- Side-by-side result and logs for simple jobs -->
			<div class="w-full flex flex-row gap-2 justify-start mb-4">
				<div class="flex-1 min-w-min">
					{#if job}
						<JobDetailHeader {job} extraCompact />
					{/if}
				</div>

				<Button variant="default" size="xs" on:click={() => onOpenDetails?.()}>Open details</Button>
			</div>

			{#if isOwner !== undefined && suspendStatus}
				<FlowExecutionStatus
					{job}
					workspaceId={$workspaceStore}
					{isOwner}
					innerModules={job?.flow_status?.modules}
					{suspendStatus}
				/>

				<div class="py-2"></div>
			{/if}

			<div class="grid grid-cols-2 gap-4 h-full min-h-[200px] max-h-[400px] w-full">
				<!-- Result Column -->
				<div class="flex flex-col min-h-0 max-h-full">
					<h3 class="text-xs font-semibold text-emphasis mb-1">Result</h3>
					<div
						class="flex-1 min-h-0 max-h-full overflow-auto rounded-md border bg-surface-tertiary p-4"
					>
						{#if job !== undefined && (job['result_stream'] || (job.type == 'CompletedJob' && 'result' in job && job.result !== undefined))}
							<DisplayResult
								workspaceId={job?.workspace_id}
								result_stream={job['result_stream']}
								jobId={job?.id}
								result={'result' in job ? job.result : undefined}
								language={job?.language}
								isTest={false}
							/>
						{:else if job}
							<div class="w-full h-full flex items-center justify-center text-secondary text-sm">
								No output is available yet
							</div>
						{:else}
							<div class="w-full h-full flex items-center justify-center">
								<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Logs Column -->
				<div class="flex flex-col min-h-0 max-h-full">
					<h3 class="text-xs font-semibold text-emphasis mb-1">Logs</h3>
					<div
						class="flex-1 min-h-0 max-h-full overflow-auto rounded-md border bg-surface-tertiary"
					>
						<LogViewer
							jobId={job.id}
							duration={job?.['duration_ms']}
							mem={job?.['mem_peak']}
							isLoading={job?.['running'] == false}
							content={job?.logs}
							tag={job?.tag}
						/>
					</div>
				</div>
			</div>
		{:else}
			<p class="p-4 text-secondary text-xs">
				The result of the flow will be the result of the last node.
			</p>
		{/if}
	</div>
</FlowCard>
