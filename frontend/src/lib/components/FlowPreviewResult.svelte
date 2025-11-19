<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	import type { FlowStatusModule, Job } from '$lib/gen'
	import { emptyString, type StateStore } from '$lib/utils'
	import Badge from './common/badge/Badge.svelte'

	interface Props {
		job: Job
		workspaceId: string | undefined
		isOwner: boolean
		hideFlowResult: boolean
		hideDownloadLogs: boolean
		innerModules: FlowStatusModule[] | undefined
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
		hideJobId?: boolean
		extra?: import('svelte').Snippet
		result_streams?: Record<string, string | undefined>
	}

	let {
		job,
		workspaceId,
		isOwner,
		hideFlowResult,
		hideDownloadLogs,
		innerModules,
		suspendStatus,
		hideJobId,
		extra,
		result_streams
	}: Props = $props()

	// Sometimes the approval form is duplicated but I can't reproduce the issue
	// This is a temporary debug log to try to catch it when it happens
	// (See the #each below)
	$effect(() =>
		console.log(
			'suspendStatusVal',
			Object.entries(suspendStatus.val || {}).map(([k, v]) => [k, v.job.id])
		)
	)
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
				downloadLogs={!hideDownloadLogs}
			/>
		</div>
	{/if}
{:else if job.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'}
	<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
{:else if suspendStatus.val && Object.keys(suspendStatus.val).length > 0}
	<div class="flex gap-2 flex-col">
		{#each Object.values(suspendStatus.val) as suspendCount (suspendCount.job.id)}
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
{:else if innerModules && innerModules?.length > 0}
	<div class="flex flex-col gap-1">
		{#each innerModules as mod, i (mod.id)}
			{#if mod.type == 'InProgress'}
				{@const rawMod = job.raw_flow?.modules[i]}

				<div>
					<span class="inline-flex gap-1">
						<Badge
							color="indigo"
							wrapperClass="max-w-full"
							baseClass="max-w-full truncate !px-1"
							title={mod.id}
						>
							<span class="max-w-full text-2xs truncate">{mod.id}</span></Badge
						>
						<span class="font-medium text-primary mt-0.5">
							{#if !emptyString(rawMod?.summary)}
								{rawMod?.summary ?? ''}
							{:else if rawMod?.value.type == 'script'}
								{rawMod.value.path ?? ''}
							{:else if rawMod?.value.type}
								{rawMod?.value.type}
							{/if}
						</span>

						<Loader2 class="animate-spin mt-0.5" /></span
					></div
				>
				{#if mod.job && result_streams?.[mod.job]}
					<pre class="text-xs text-primary">{result_streams?.[mod.job]}</pre>
				{/if}
			{/if}
		{/each}
	</div>
{/if}
