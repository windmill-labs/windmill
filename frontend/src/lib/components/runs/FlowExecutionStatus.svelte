<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import FlowStatusWaitingForEvents from '../FlowStatusWaitingForEvents.svelte'
	import type { FlowStatusModule, Job } from '$lib/gen'
	import { emptyString, type StateStore } from '$lib/utils'
	import Badge from '../common/badge/Badge.svelte'
	import { fade, slide } from 'svelte/transition'
	import { untrack } from 'svelte'

	interface Props {
		job: Job
		workspaceId: string | undefined
		isOwner: boolean
		innerModules: FlowStatusModule[] | undefined
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
		result_streams?: Record<string, string | undefined>
	}

	let { job, workspaceId, isOwner, innerModules, suspendStatus, result_streams }: Props = $props()

	const isWaitingForEvents = $derived(
		job?.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'
	)
	const isSuspended = $derived(suspendStatus.val && Object.keys(suspendStatus.val).length > 0)
	const hasInnerModulesInProgress = $derived(
		job?.flow_status?.modules?.some((mod) => mod.type === 'InProgress')
	)

	let shouldShowAnyStatus = $state(false)
	let debounceTimer: number | undefined = $state()

	const anyStatusActive = $derived(isWaitingForEvents || isSuspended || hasInnerModulesInProgress)

	$effect(() => {
		if (anyStatusActive) {
			// If any status becomes active, show immediately
			untrack(() => {
				shouldShowAnyStatus = true
				if (debounceTimer) {
					clearTimeout(debounceTimer)
					debounceTimer = undefined
				}
			})
		} else {
			// If no status is active, wait 200ms before hiding
			untrack(() => {
				if (debounceTimer) {
					clearTimeout(debounceTimer)
				}
				debounceTimer = setTimeout(() => {
					shouldShowAnyStatus = false
					debounceTimer = undefined
				}, 200)
			})
		}

		// Cleanup function
		return () => {
			if (debounceTimer) {
				clearTimeout(debounceTimer)
				debounceTimer = undefined
			}
		}
	})
</script>

{#if job && shouldShowAnyStatus}
	<div
		class="rounded-md border shadow-md p-4 bg-surface-tertiary transition-all duration-150 flex flex-col justify-center"
	>
		{#if isWaitingForEvents}
			<div in:slide={{ duration: 150 }}>
				<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
			</div>
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
		{:else if hasInnerModulesInProgress}
			<div class="flex flex-col gap-1" in:fade={{ duration: 150 }}>
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
	</div>
{/if}
