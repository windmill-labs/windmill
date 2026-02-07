<script lang="ts">
	import FlowStatusWaitingForEvents from '../FlowStatusWaitingForEvents.svelte'
	import type { FlowStatusModule, Job } from '$lib/gen'
	import { type StateStore } from '$lib/utils'
	import { fade, slide } from 'svelte/transition'

	interface Props {
		job: Job
		workspaceId: string | undefined
		isOwner: boolean
		innerModules: FlowStatusModule[] | undefined
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
	}

	let { job, workspaceId, isOwner, suspendStatus }: Props = $props()

	const isWaitingForEvents = $derived(
		job?.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'
	)
	const isSuspended = $derived(suspendStatus.val && Object.keys(suspendStatus.val).length > 0)

	const anyStatusActive = $derived(isWaitingForEvents || isSuspended)
</script>

{#if job && anyStatusActive}
	<div
		class="rounded-md border shadow-md p-4 bg-surface-tertiary flex flex-col justify-center"
		transition:slide={{ duration: 150 }}
	>
		{#if isWaitingForEvents}
			<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
		{:else if isSuspended}
			<div class="flex gap-2 flex-col" in:fade={{ duration: 150 }}>
				{#each Object.values(suspendStatus.val) as suspendCount (suspendCount.job.id)}
					<div>
						<div class="text-sm">
							Flow suspended, waiting for {suspendCount.nb} events
						</div>
						<FlowStatusWaitingForEvents job={suspendCount.job} {workspaceId} {isOwner} />
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
