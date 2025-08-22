<script lang="ts">
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { FlowModule } from '$lib/gen/types.gen'
	import { ContextIconMap, type ContextElement } from './context'
	import { ArrowLeft, Diff, Database, Code, ChevronRight } from 'lucide-svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		showAllAvailable?: boolean
		stringSearch?: string
		selectedIndex?: number
		onKeyDown?: (e: KeyboardEvent) => void
		onNavigate?: (direction: 'up' | 'down' | 'enter' | 'back') => void
		isActive?: boolean
		setItemsNumber?: (newNumber: number) => void
	}

	const {
		availableContext,
		selectedContext,
		onSelect,
		showAllAvailable = false,
		stringSearch = '',
		selectedIndex = 0,
		onKeyDown,
		onNavigate,
		isActive = true,
		setItemsNumber
	}: Props = $props()

	// Current view state: 'categories' or specific category type
	let currentView = $state<'categories' | 'diffs' | 'modules' | 'databases' | 'code'>('categories')

	// Selected index for keyboard navigation
	let categorySelectedIndex = $state(0)
	let itemSelectedIndex = $state(0)

	// Category definitions
	const categories = [
		{ id: 'diffs', label: 'Diffs', icon: Diff },
		{ id: 'modules', label: 'Modules', icon: BarsStaggered },
		{ id: 'databases', label: 'Databases', icon: Database },
		{ id: 'code', label: 'Code', icon: Code }
	]

	const filteredAvailableContext = $derived(
		availableContext.filter((context) => {
			const filtered =
				(showAllAvailable ||
					!selectedContext.some((sc) => sc.type === context.type && sc.title === context.title)) &&
				(!stringSearch || context.title.toLowerCase().includes(stringSearch.toLowerCase()))

			return filtered
		})
	)

	// Group context by category
	const contextByCategory = $derived.by(() => {
		const grouped: Record<string, ContextElement[]> = {
			diffs: [],
			modules: [],
			databases: [],
			code: []
		}

		filteredAvailableContext.forEach((context) => {
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

	// Filter to only show categories with items
	const availableCategories = $derived(
		categories.filter((cat) => contextByCategory[cat.id].length > 0)
	)

	// Update selected indices when selectedIndex prop changes
	$effect(() => {
		if (selectedIndex !== undefined && currentView !== 'categories') {
			itemSelectedIndex = selectedIndex
		}
	})

	// Reset item index when changing categories or search changes
	$effect(() => {
		itemSelectedIndex = 0
	})

	// Report view changes
	$effect(() => {
		if (setItemsNumber) {
			if (currentView === 'categories') {
				setItemsNumber(availableCategories.length)
			} else {
				setItemsNumber(currentCategoryItems.length + 1)
			}
		}
	})

	function handleCategoryClick(categoryId: string) {
		currentView = categoryId as typeof currentView
		itemSelectedIndex = 0
	}

	function handleBackClick() {
		currentView = 'categories'
		itemSelectedIndex = 0
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!isActive) return

		let handled = false

		if (stringSearch.length > 0) {
			// Navigation in search view (flat list)
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				if (filteredAvailableContext.length > 0) {
					itemSelectedIndex = (itemSelectedIndex + 1) % filteredAvailableContext.length
				}
				handled = true
				onNavigate?.('down')
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				if (filteredAvailableContext.length > 0) {
					itemSelectedIndex =
						(itemSelectedIndex - 1 + filteredAvailableContext.length) %
						filteredAvailableContext.length
				}
				handled = true
				onNavigate?.('up')
			} else if (e.key === 'Enter' || e.key === 'Tab') {
				if (e.key === 'Tab') e.preventDefault()
				e.stopPropagation()
				const selectedItem = filteredAvailableContext[itemSelectedIndex]
				if (selectedItem) {
					onSelect(selectedItem)
				}
				handled = true
				onNavigate?.('enter')
			}
		} else if (currentView === 'categories') {
			// Navigation in categories view
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				categorySelectedIndex = (categorySelectedIndex + 1) % availableCategories.length
				handled = true
				onNavigate?.('down')
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				categorySelectedIndex =
					(categorySelectedIndex - 1 + availableCategories.length) % availableCategories.length
				handled = true
				onNavigate?.('up')
			} else if (e.key === 'Enter' || e.key === 'ArrowRight') {
				e.preventDefault()
				e.stopPropagation()
				const selectedCategory = availableCategories[categorySelectedIndex]
				if (selectedCategory) {
					handleCategoryClick(selectedCategory.id)
				}
				handled = true
				onNavigate?.('enter')
			}
		} else {
			// Navigation in category items view
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				if (currentCategoryItems.length > 0) {
					itemSelectedIndex = (itemSelectedIndex + 1) % currentCategoryItems.length
				}
				handled = true
				onNavigate?.('down')
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				if (currentCategoryItems.length > 0) {
					itemSelectedIndex =
						(itemSelectedIndex - 1 + currentCategoryItems.length) % currentCategoryItems.length
				}
				handled = true
				onNavigate?.('up')
			} else if (e.key === 'Enter' || e.key === 'Tab') {
				if (e.key === 'Tab') e.preventDefault()
				e.stopPropagation()
				const selectedItem = currentCategoryItems[itemSelectedIndex]
				if (selectedItem) {
					onSelect(selectedItem)
					currentView = 'categories' // Go back to categories after selection
				}
				handled = true
				onNavigate?.('enter')
			} else if (e.key === 'ArrowLeft' || e.key === 'Escape') {
				e.preventDefault()
				e.stopPropagation()
				handleBackClick()
				handled = true
				onNavigate?.('back')
			}
		}

		// Pass through unhandled keys to parent
		if (!handled && onKeyDown) {
			onKeyDown(e)
		}
	}

	// Listen for keyboard events
	$effect(() => {
		if (isActive) {
			document.addEventListener('keydown', handleKeyDown)
			return () => {
				document.removeEventListener('keydown', handleKeyDown)
			}
		}
	})
</script>

<div class="flex flex-col gap-1 text-tertiary text-xs p-1 pr-0 min-w-24 max-h-48 overflow-y-scroll">
	{#if stringSearch.length > 0}
		<!-- Search view - show flat list -->
		{#each filteredAvailableContext as element, i}
			{@const Icon = ContextIconMap[element.type]}
			<button
				class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal transition-colors {i ===
				itemSelectedIndex
					? 'bg-surface-hover'
					: ''}"
				onclick={() => {
					onSelect(element)
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
		{#if filteredAvailableContext.length === 0}
			<div class="text-center text-tertiary text-xs py-2">No matching context</div>
		{/if}
	{:else if currentView === 'categories'}
		<!-- Categories view -->
		{#each availableCategories as category, i}
			{@const Icon = category.icon}
			<button
				class="hover:bg-surface-hover rounded-md p-1 pr-0 text-left flex flex-row gap-1 items-center font-normal transition-colors {i ===
				categorySelectedIndex
					? 'bg-surface-hover'
					: ''}"
				onclick={() => handleCategoryClick(category.id)}
			>
				<Icon size={16} />
				<span class="flex-1">{category.label}</span>
				<ChevronRight size={16} />
			</button>
		{/each}
		{#if availableCategories.length === 0}
			<div class="text-center text-tertiary text-xs py-2">No available context</div>
		{/if}
	{:else}
		<!-- Category items view -->
		<button
			class="hover:bg-surface-hover rounded-md text-left flex flex-row gap-1 items-center font-normal transition-colors mb-1"
			onclick={handleBackClick}
		>
			<ArrowLeft size={12} />
			<span class="text-xs">Go back</span>
		</button>

		{#if currentCategoryItems.length === 0}
			<div class="text-center text-tertiary text-xs py-2">No items in this category</div>
		{:else}
			{#each currentCategoryItems as element, i}
				{@const Icon = ContextIconMap[element.type]}
				<button
					class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal transition-colors {i ===
					itemSelectedIndex
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
	{/if}
</div>
