<script lang="ts">
	import { ContextIconMap, type ContextElement } from './context'
	import { ArrowLeft, ChevronRight, FileCode, Code2, Table2 } from 'lucide-svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		setShowing?: (showing: boolean) => void
		showAllAvailable?: boolean
		stringSearch?: string
		onViewChange?: (newNumber: number) => void
	}

	const {
		availableContext,
		selectedContext,
		onSelect,
		setShowing,
		showAllAvailable = false,
		stringSearch = '',
		onViewChange
	}: Props = $props()

	// Current view state: 'categories' or specific category type
	let currentView = $state<'categories' | 'files' | 'runnables' | 'datatables'>('categories')

	// Selected index for keyboard navigation
	let itemSelectedIndex = $state(0)
	let categorySelectedIndex = $state(0)

	// Category definitions for app mode
	const categories = [
		{ id: 'files', label: 'Frontend Files', icon: FileCode },
		{ id: 'runnables', label: 'Backend Runnables', icon: Code2 },
		{ id: 'datatables', label: 'Datatables', icon: Table2 }
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
			files: [],
			runnables: [],
			datatables: []
		}

		filteredAvailableContext.forEach((context) => {
			if (context.type === 'app_frontend_file') grouped.files.push(context)
			else if (context.type === 'app_backend_runnable') grouped.runnables.push(context)
			else if (context.type === 'app_datatable') grouped.datatables.push(context)
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

	// Report view changes
	$effect(() => {
		if (onViewChange) {
			if (currentView === 'categories') {
				onViewChange(availableCategories.length)
			} else {
				onViewChange(currentCategoryItems.length + 1)
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
		if (stringSearch.length > 0) {
			// Navigation in search view (flat list)
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				if (filteredAvailableContext.length > 0) {
					itemSelectedIndex = (itemSelectedIndex + 1) % filteredAvailableContext.length
				}
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				if (filteredAvailableContext.length > 0) {
					itemSelectedIndex =
						(itemSelectedIndex - 1 + filteredAvailableContext.length) %
						filteredAvailableContext.length
				}
			} else if (e.key === 'Enter' || e.key === 'Tab') {
				if (e.key === 'Tab') e.preventDefault()
				e.stopPropagation()
				const selectedItem = filteredAvailableContext[itemSelectedIndex]
				if (selectedItem) {
					onSelect(selectedItem)
				}
			}
		} else if (currentView === 'categories') {
			// Navigation in categories view
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				categorySelectedIndex = (categorySelectedIndex + 1) % availableCategories.length
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				categorySelectedIndex =
					(categorySelectedIndex - 1 + availableCategories.length) % availableCategories.length
			} else if (e.key === 'Enter' || e.key === 'ArrowRight' || e.key === 'Tab') {
				e.preventDefault()
				e.stopPropagation()
				const selectedCategory = availableCategories[categorySelectedIndex]
				if (selectedCategory) {
					handleCategoryClick(selectedCategory.id)
				}
			} else if (e.key === 'Escape' || e.key === 'ArrowLeft') {
				e.preventDefault()
				e.stopPropagation()
				setShowing?.(false)
			}
		} else {
			// Navigation in category items view
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				if (currentCategoryItems.length > 0) {
					itemSelectedIndex = (itemSelectedIndex + 1) % currentCategoryItems.length
				}
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				if (currentCategoryItems.length > 0) {
					itemSelectedIndex =
						(itemSelectedIndex - 1 + currentCategoryItems.length) % currentCategoryItems.length
				}
			} else if (e.key === 'Enter' || e.key === 'Tab') {
				if (e.key === 'Tab') e.preventDefault()
				e.stopPropagation()
				const selectedItem = currentCategoryItems[itemSelectedIndex]
				if (selectedItem) {
					onSelect(selectedItem)
					currentView = 'categories' // Go back to categories after selection
				}
			} else if (e.key === 'ArrowLeft' || e.key === 'Escape') {
				e.preventDefault()
				e.stopPropagation()
				handleBackClick()
			}
		}
	}

	// Listen for keyboard events
	$effect(() => {
		document.addEventListener('keydown', handleKeyDown)
		return () => {
			document.removeEventListener('keydown', handleKeyDown)
		}
	})

	$effect(() => {
		if (stringSearch.length > 0) {
			itemSelectedIndex = 0
		}
	})

	function getDisplayTitle(element: ContextElement): string {
		if (element.type === 'app_frontend_file') {
			return element.path
		} else if (element.type === 'app_backend_runnable') {
			return element.key
		} else if (element.type === 'app_datatable') {
			// Show as datatable/table or datatable/schema:table
			return element.title
		}
		return element.title
	}
</script>

<div
	class="flex flex-col gap-1 text-primary text-xs p-1 pr-0 min-w-24 max-h-48 overflow-y-scroll"
	onmousedown={(e) =>
		// avoids triggering onblur on the textinput and closing the tooltip
		e.preventDefault()}
	role="listbox"
	tabindex={0}
>
	{#if stringSearch.length > 0}
		<!-- Search view - show flat list -->
		{#each filteredAvailableContext as element, i (element.type + '-' + element.title)}
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
				{#if Icon}
					<Icon size={16} />
				{/if}
				<span class="truncate">
					{getDisplayTitle(element)}
				</span>
			</button>
		{/each}
		{#if filteredAvailableContext.length === 0}
			<div class="text-center text-primary text-xs py-2">No matching context</div>
		{/if}
	{:else if currentView === 'categories'}
		<!-- Categories view -->
		{#each availableCategories as category, i (category.id)}
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
			<div class="text-center text-primary text-xs py-2">No available context</div>
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
			<div class="text-center text-primary text-xs py-2">No items in this category</div>
		{:else}
			{#each currentCategoryItems as element, i (element.type + '-' + element.title)}
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
					{#if Icon}
						<Icon size={16} />
					{/if}
					<span class="truncate">
						{getDisplayTitle(element)}
					</span>
				</button>
			{/each}
		{/if}
	{/if}
</div>
