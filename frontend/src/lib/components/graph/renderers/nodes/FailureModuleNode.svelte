<script lang="ts">
	import { Bug, EyeOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import NodeWrapper from './NodeWrapper.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import type { FailureModuleN } from '../../graphBuilder.svelte'
	import { getNodeColorClasses, NODE } from '$lib/components/graph'
	import { msToSec } from '$lib/utils'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	interface Props {
		data: FailureModuleN['data']
	}

	let { data }: Props = $props()
	const flowRunStatus = getFlowRunStatusContext()

	let state = $derived(flowRunStatus?.getModuleState(data.id))
	let colorClasses = $derived(getNodeColorClasses(state?.skipped ? '_Skipped' : state?.type, false))
</script>

<NodeWrapper>
	{#if state?.duration_ms}
		<div
			class="absolute z-5 right-0 -top-4 mr-2 center-center text-2xs font-normal text-gray-400 dark:text-gray-500"
		>
			{msToSec(state.duration_ms)}s
		</div>
	{/if}
	<div
		class="flex flex-row justify-center items-center"
		style="width: {NODE.width}px; height: {NODE.height}px;"
	>
		<div class="group relative">
			<Tooltip placement="bottom">
				<div
					class={twMerge(
						'flex flex-row items-center gap-2 rounded-md border border-dashed border-border-normal px-3 py-1.5 max-w-full cursor-default',
						colorClasses.bg
					)}
				>
					<Bug size={14} class={twMerge('shrink-0', colorClasses.text)} />
					<div class={twMerge('truncate text-2xs', colorClasses.text)}>
						{data.module.summary || 'Error handler'}
					</div>
				</div>
				{#snippet text()}
					Marks where the error handler ran during the last run. It is not a step of the flow: edit
					it from the error handler button above the graph.
				{/snippet}
			</Tooltip>
			<!-- Deliberately not an `X` on a bug badge: that is the header control which deletes
			`failure_module` for good. This one only hides a run marker. -->
			<Tooltip
				placement="top"
				class="absolute -top-1.5 -right-1.5 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
			>
				<button
					type="button"
					aria-label="Hide error handler run marker"
					class="rounded-full border border-border bg-surface p-0.5 text-secondary hover:bg-surface-hover"
					onclick={() => data.eventHandlers.dismissRunNode(data.id)}
				>
					<EyeOff size={11} />
				</button>
				{#snippet text()}
					Hide this marker until the next run. The error handler itself is kept.
				{/snippet}
			</Tooltip>
		</div>
	</div>
</NodeWrapper>
