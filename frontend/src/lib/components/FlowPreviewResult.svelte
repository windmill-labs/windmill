<script lang="ts">
	import { Badge, Loader2 } from 'lucide-svelte'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	import type { FlowStatusModule, Job } from '$lib/gen'
	import { emptyString } from '$lib/utils'
	import type { DurationStatus } from './graph'
	import type { Writable } from 'svelte/store'

	interface Props {
		job: Job
		workspaceId: string | undefined
		isOwner: boolean
		hideFlowResult: boolean
		hideDownloadLogs: boolean
		localDurationStatuses: Writable<Record<string, DurationStatus>>
		innerModules: FlowStatusModule[]
		suspendStatus: Writable<Record<string, { job: Job; nb: number }>>
		hideJobId?: boolean
		extra?: import('svelte').Snippet
	}

	let {
		job,
		workspaceId,
		isOwner,
		hideFlowResult,
		hideDownloadLogs,
		localDurationStatuses,
		innerModules,
		suspendStatus,
		hideJobId,
		extra
	}: Props = $props()
</script>

<FlowPreviewStatus {job} {hideJobId} {extra} />

{#if !job}
	<div>
		<Loader2 class="animate-spin" />
	</div>
{:else if `result` in job}
	{#if !hideFlowResult}
		<div class="w-full h-full">
			<FlowJobResult
				workspaceId={job?.workspace_id}
				jobId={job?.id}
				tag={job?.tag}
				loading={job['running'] == true}
				result={job.result}
				logs={job.logs}
				durationStates={localDurationStatuses}
				downloadLogs={!hideDownloadLogs}
			/>
		</div>
	{/if}
{:else if job.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'}
	<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
{:else if $suspendStatus && Object.keys($suspendStatus).length > 0}
	<div class="flex gap-2 flex-col">
		{#each Object.values($suspendStatus) as suspendCount (suspendCount.job.id)}
			<div>
				<div class="text-sm">
					Flow suspended, waiting for {suspendCount.nb} events
				</div>
				<FlowStatusWaitingForEvents job={suspendCount.job} {workspaceId} {isOwner} />
			</div>
		{/each}
	</div>
{:else if job.logs}
	<div
		class="text-xs p-4 bg-surface-secondary overflow-auto max-h-80 border border-tertiary-inverse"
	>
		<pre class="w-full">{job.logs}</pre>
	</div>
{:else if innerModules?.length > 0}
	<div class="flex flex-col gap-1">
		{#each innerModules as mod, i (mod.id)}
			{#if mod.type == 'InProgress'}
				{@const rawMod = job.raw_flow?.modules[i]}

				<div>
					<span class="inline-flex gap-1">
						<span class="font-medium text-primary">
							{#if !emptyString(rawMod?.summary)}
								{rawMod?.summary ?? ''}
							{:else if rawMod?.value.type == 'script'}
								{rawMod.value.path ?? ''}
							{:else if rawMod?.value.type}
								{rawMod?.value.type}
							{/if}
						</span>

						<Loader2 class="animate-spin" /></span
					></div
				>
			{/if}
		{/each}
	</div>
{/if}
