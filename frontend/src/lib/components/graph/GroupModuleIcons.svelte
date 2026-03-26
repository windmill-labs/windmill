<script lang="ts">
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { getGraphContext } from './graphContext'
	import { getNodeColorClasses } from '$lib/components/graph'
	import type { FlowNodeState } from '$lib/components/graph/util'
	import type { GraphModuleState } from './model'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from './graphBuilder.svelte'

	interface Props {
		modules: FlowModule[]
		flowModuleStates?: Record<string, GraphModuleState> | undefined
		eventHandlers?: GraphEventHandlers
	}

	let { modules, flowModuleStates, eventHandlers }: Props = $props()

	const { selectionManager } = getGraphContext()

	// Badge width model: icon(16) + pl(2) + gap(2) + pr(6) = 26px fixed + ~5.5px per char
	const BADGE_FIXED = 26
	const CHAR_WIDTH = 5.5
	const BADGE_MAX = 128 // 8rem
	const GAP = 4 // gap-1
	const ROW_WIDTH = 255 // 275px container - 2*px-2 padding
	const OVERFLOW_BTN_WIDTH = 32

	function estimateBadgeWidth(id: string): number {
		return BADGE_FIXED + id.length * CHAR_WIDTH
	}

	function totalWidth(mods: FlowModule[], capped: boolean): number {
		return mods.reduce(
			(sum, mod, i) =>
				sum +
				Math.min(estimateBadgeWidth(mod.id), capped ? BADGE_MAX : Infinity) +
				(i > 0 ? GAP : 0),
			0
		)
	}

	let { displayModules, overflowModules, capIds } = $derived.by(() => {
		// Step 1: try all badges with full ids
		if (totalWidth(modules, false) <= ROW_WIDTH) {
			return { displayModules: modules, overflowModules: [], capIds: false }
		}

		// Step 2: try all badges with ids capped at 8rem
		if (totalWidth(modules, true) <= ROW_WIDTH) {
			return { displayModules: modules, overflowModules: [], capIds: true }
		}

		// Step 3: remove last modules until capped badges fit
		const available = ROW_WIDTH - OVERFLOW_BTN_WIDTH - GAP
		let count = modules.length
		while (count > 1 && totalWidth(modules.slice(0, count), true) > available) {
			count--
		}

		return {
			displayModules: modules.slice(0, count),
			overflowModules: modules.slice(count),
			capIds: true
		}
	})

	const STATE_PRIORITY: Record<string, number> = {
		WaitingForEvents: 4,
		InProgress: 3,
		WaitingForExecutor: 3,
		Failure: 2,
		Success: 1
	}

	let overflowAggregateState = $derived.by<FlowNodeState | undefined>(() => {
		if (!flowModuleStates) return undefined
		let best: FlowNodeState | undefined = undefined
		let bestPriority = 0
		for (const mod of overflowModules) {
			const state = flowModuleStates[mod.id]?.type
			if (state) {
				const p = STATE_PRIORITY[state] ?? 0
				if (p > bestPriority) {
					bestPriority = p
					best = state
				}
			}
		}
		return best
	})

	function moduleLabel(mod: FlowModule): string {
		if (mod.summary) return mod.summary
		const type = mod.value?.type
		if (type === 'forloopflow') return 'For loop'
		if (type === 'whileloopflow') return 'While loop'
		if (type === 'branchone') return 'Run one branch'
		if (type === 'branchall') return 'Run all branches'
		if (type === 'flow') return 'Flow'
		if (type === 'identity') return 'Identity'
		if (type === 'aiagent') return 'AI Agent'
		return mod.id
	}

	function selectModule(mod: FlowModule) {
		selectionManager.selectId(mod.id)
		eventHandlers?.select(mod.id)
	}
</script>

<div class="flex items-center gap-1">
	{#each displayModules as mod (mod.id)}
		{@const selected = selectionManager.isNodeSelected(mod.id)}
		{@const nodeState = flowModuleStates?.[mod.id]?.type}
		{@const colorClasses = getNodeColorClasses(nodeState, selected)}
		<Tooltip placement="bottom">
			{#snippet children()}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="h-5 rounded-md overflow-hidden flex items-center gap-0.5 shrink-0 shadow-sm pl-0.5 pr-1.5 cursor-pointer hover:opacity-80 {colorClasses.bg} {colorClasses.outline}"
					style={capIds ? 'max-width: 8rem;' : ''}
					onclick={() => selectModule(mod)}
				>
					<div class="w-4 h-4 flex items-center justify-center shrink-0">
						<FlowModuleIcon module={mod} size={12} />
					</div>
					<span class="text-3xs font-medium truncate text-right flex-1 {colorClasses.text}"
						>{mod.id}</span
					>
				</div>
			{/snippet}
			{#snippet text()}
				<span class="font-medium">{mod.id}</span>: {moduleLabel(mod)}
			{/snippet}
		</Tooltip>
	{/each}
	{#if overflowModules.length > 0}
		{@const overflowColorClasses = getNodeColorClasses(overflowAggregateState, false)}
		<DropdownV2 placement="bottom" customMenu usePointerDownOutside>
			{#snippet buttonReplacement()}
				<div
					class="h-5 rounded-md flex items-center justify-center shrink-0 shadow-sm px-1.5 cursor-pointer hover:opacity-80 {overflowColorClasses.bg} {overflowColorClasses.outline}"
				>
					<span class="text-3xs font-medium {overflowColorClasses.text}"
						>+{overflowModules.length}</span
					>
				</div>
			{/snippet}
			{#snippet menu()}
				<div
					class="bg-surface-tertiary dark:border rounded-lg shadow-lg py-1 w-56 overflow-y-auto"
					style="max-height: 50vh;"
				>
					{#each overflowModules as mod (mod.id)}
						{@const nodeState = flowModuleStates?.[mod.id]?.type}
						{@const colorClasses = getNodeColorClasses(nodeState, false)}
						{@const selected = selectionManager.isNodeSelected(mod.id)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-surface-hover text-2xs {selected
								? 'bg-surface-accent-selected'
								: ''}"
							onclick={() => selectModule(mod)}
						>
							<div
								class="w-4 h-4 rounded flex items-center justify-center shrink-0 {colorClasses.bg}"
							>
								<FlowModuleIcon module={mod} size={12} />
							</div>
							<span class="truncate flex-1">{moduleLabel(mod)}</span>
							<span class="text-tertiary shrink-0">{mod.id}</span>
						</div>
					{/each}
				</div>
			{/snippet}
		</DropdownV2>
	{/if}
</div>
