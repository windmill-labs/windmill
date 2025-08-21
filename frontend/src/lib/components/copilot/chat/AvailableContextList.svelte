<script lang="ts">
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { FlowModule } from '$lib/gen/types.gen'
	import { ContextIconMap, type ContextElement } from './context'
	import { ArrowLeft, Diff, Database, Code } from 'lucide-svelte'

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

	// Current view state: 'categories' or specific category type
	let currentView = $state<'categories' | 'diffs' | 'modules' | 'databases' | 'code'>('categories')

	// Category definitions
	const categories = [
		{ id: 'diffs', label: 'Diffs', icon: Diff },
		{ id: 'modules', label: 'Modules', icon: BarsStaggered },
		{ id: 'databases', label: 'Databases', icon: Database },
		{ id: 'code', label: 'Code', icon: Code }
	]

	// Group context by category
	const contextByCategory = $derived.by(() => {
		const grouped: Record<string, ContextElement[]> = {
			diffs: [],
			modules: [],
			databases: [],
			code: []
		}

		availableContext.forEach((context) => {
			const filtered =
				(showAllAvailable ||
					!selectedContext.some((sc) => sc.type === context.type && sc.title === context.title)) &&
				(!stringSearch || context.title.toLowerCase().includes(stringSearch.toLowerCase()))

			if (!filtered) return

			if (context.type === 'diff') grouped.diffs.push(context)
			else if (context.type === 'flow_module') grouped.modules.push(context)
			else if (context.type === 'db') grouped.databases.push(context)
			else if (context.type === 'code') grouped.code.push(context)
		})

		return grouped
	})

	const currentCategoryItems = $derived(
		currentView !== 'categories' ? contextByCategory[currentView] : []
	)

	function handleCategoryClick(categoryId: string) {
		currentView = categoryId as typeof currentView
	}

	function handleBackClick() {
		currentView = 'categories'
	}
</script>

<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24 max-h-48 overflow-y-scroll">
	{#if currentView === 'categories'}
		{#each categories as category}
			{@const itemCount = contextByCategory[category.id].length}
			{@const Icon = category.icon}
			{#if itemCount > 0}
				<button
					class="hover:bg-surface-hover rounded-md p-2 text-left flex flex-row gap-2 items-center font-normal transition-colors"
					onclick={() => handleCategoryClick(category.id)}
				>
					<Icon size={16} />
					<span class="flex-1">{category.label}</span>
				</button>
			{/if}
		{/each}
		{#if categories.every((cat) => contextByCategory[cat.id].length === 0)}
			<div class="text-center text-tertiary text-xs py-2">No available context</div>
		{/if}
	{:else}
		<!-- Category items view -->
		<div class="flex flex-col gap-1">
			<button
				class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal mb-1 transition-colors"
				onclick={handleBackClick}
			>
				<ArrowLeft size={14} />
				<span>Back</span>
			</button>

			{#if currentCategoryItems.length === 0}
				<div class="text-center text-tertiary text-xs py-2">No items in this category</div>
			{:else}
				{#each currentCategoryItems as element, i}
					{@const Icon = ContextIconMap[element.type]}
					<button
						class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal transition-colors {i ===
						selectedIndex
							? 'bg-surface-hover'
							: ''}"
						onclick={() => {
							onSelect(element)
							currentView = 'categories' // Go back to categories after selection
						}}
					>
						{#if element.type === 'flow_module'}
							<FlowModuleIcon module={element as FlowModule} size={16} />
						{:else if Icon}
							<Icon size={16} />
						{/if}
						<span class="truncate">
							{element.type === 'diff' || element.type === 'flow_module'
								? element.title.replace(/_/g, ' ')
								: element.title}
						</span>
					</button>
				{/each}
			{/if}
		</div>
	{/if}
</div>
