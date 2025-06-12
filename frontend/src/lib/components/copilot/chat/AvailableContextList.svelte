<script lang="ts">
	import { ContextIconMap, type ContextElement } from './context'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		showAllAvailable?: boolean
		stringSearch?: string
		selectedIndex?: number
	}

	const {
		availableContext,
		selectedContext,
		onSelect,
		showAllAvailable = false,
		stringSearch = '',
		selectedIndex = 0
	}: Props = $props()

	// Define priority map for context types
	const typePriority = {
		code: 1,
		diff: 2,
		default: 3
	}

	const sortedAvailableContext = $derived.by(() => {
		let copy = [...availableContext]
		copy.sort((a, b) => {
			const priorityA = typePriority[a.type] || typePriority.default
			const priorityB = typePriority[b.type] || typePriority.default
			return priorityA - priorityB
		})
		return copy
	})

	const actualAvailableContext = $derived(
		showAllAvailable
			? sortedAvailableContext.filter(
					(c) => !stringSearch || c.title.toLowerCase().includes(stringSearch.toLowerCase())
				)
			: sortedAvailableContext.filter(
					(c) =>
						!selectedContext.find((sc) => sc.type === c.type && sc.title === c.title) &&
						(!stringSearch || c.title.toLowerCase().includes(stringSearch.toLowerCase()))
				)
	)
</script>

<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24 max-h-48 overflow-y-scroll">
	{#if actualAvailableContext.length === 0}
		<div class="text-center text-tertiary text-xs">No available context</div>
	{:else}
		{#each actualAvailableContext as element, i}
			{@const Icon = ContextIconMap[element.type]}
			<button
				class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal {i ===
				selectedIndex
					? 'bg-surface-hover'
					: ''}"
				onclick={() => onSelect(element)}
			>
				{#if Icon}
					<Icon size={16} />
				{/if}
				{element.type === 'diff' ? element.title.replace(/_/g, ' ') : element.title}
			</button>
		{/each}
	{/if}
</div>
