<script lang="ts">
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { FlowModule } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { workspaceRunnablesSearch, MAX_RUNNABLE_CONTENT_LENGTH } from './shared'
	import {
		ContextIconMap,
		type ContextElement,
		type WorkspaceScriptElement,
		type WorkspaceFlowElement
	} from './context'
	import {
		ArrowLeft,
		Diff,
		Database,
		ChevronRight,
		Code2,
		Loader2
	} from 'lucide-svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		onSelect: (element: ContextElement) => void
		onSelectWorkspaceItem?: (element: ContextElement) => void
		setShowing?: (showing: boolean) => void
		showAllAvailable?: boolean
		stringSearch?: string
		onViewChange?: (newNumber: number) => void
	}

	const {
		availableContext,
		selectedContext,
		onSelect,
		onSelectWorkspaceItem,
		setShowing,
		showAllAvailable = false,
		stringSearch = '',
		onViewChange
	}: Props = $props()

	// Current view state: 'categories' or specific category type
	let currentView = $state<
		'categories' | 'diffs' | 'modules' | 'databases' | 'scripts' | 'flows'
	>('categories')

	// Selected index for keyboard navigation
	let itemSelectedIndex = $state(0)
	let categorySelectedIndex = $state(0)

	// Workspace search state
	let workspaceSearchQuery = $state('')
	let workspaceSearchResults = $state<{ path: string; summary: string }[]>([])
	let workspaceSearchLoading = $state(false)
	let searchInputElement = $state<HTMLInputElement | undefined>(undefined)
	let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined = undefined

	// Category definitions
	const categories = [
		{ id: 'diffs' as const, label: 'Diffs', icon: Diff, searchable: false },
		{ id: 'modules' as const, label: 'Modules', icon: BarsStaggered, searchable: false },
		{ id: 'databases' as const, label: 'Databases', icon: Database, searchable: false },
		{ id: 'scripts' as const, label: 'Scripts', icon: Code2, searchable: true },
		{ id: 'flows' as const, label: 'Flows', icon: BarsStaggered, searchable: true }
	]

	const isSearchableView = $derived(
		currentView === 'scripts' || currentView === 'flows'
	)

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
			databases: []
		}

		filteredAvailableContext.forEach((context) => {
			if (context.type === 'diff') grouped.diffs.push(context)
			else if (context.type === 'flow_module') grouped.modules.push(context)
			else if (context.type === 'db') grouped.databases.push(context)
		})

		return grouped
	})

	const currentCategoryItems = $derived(
		currentView !== 'categories' && !isSearchableView ? contextByCategory[currentView] : []
	)

	// Filter to only show categories with items (non-searchable) or always show (searchable)
	const availableCategories = $derived(
		categories.filter(
			(cat) => cat.searchable || contextByCategory[cat.id]?.length > 0
		)
	)

	// Report view changes
	$effect(() => {
		if (onViewChange) {
			if (currentView === 'categories') {
				onViewChange(availableCategories.length)
			} else if (isSearchableView) {
				onViewChange(workspaceSearchResults.length + 2) // +2 for back button and search input
			} else {
				onViewChange(currentCategoryItems.length + 1)
			}
		}
	})

	function handleCategoryClick(categoryId: string) {
		currentView = categoryId as typeof currentView
		itemSelectedIndex = 0
		if (categoryId === 'scripts' || categoryId === 'flows') {
			workspaceSearchQuery = ''
			workspaceSearchResults = []
			setTimeout(() => searchInputElement?.focus(), 0)
		}
	}

	function handleBackClick() {
		currentView = 'categories'
		itemSelectedIndex = 0
		workspaceSearchQuery = ''
		workspaceSearchResults = []
	}

	async function searchWorkspaceItems(query: string) {
		const workspace = $workspaceStore
		if (!workspace) return

		workspaceSearchLoading = true
		try {
			const type = currentView === 'scripts' ? 'scripts' : 'flows'
			const results = await workspaceRunnablesSearch.search(query, workspace, type)
			workspaceSearchResults = results.map((r) => ({
				path: r.path,
				summary: r.summary
			}))
		} catch (err) {
			console.error('Error searching workspace items', err)
			workspaceSearchResults = []
		} finally {
			workspaceSearchLoading = false
		}
	}

	function handleSearchInput() {
		if (searchDebounceTimer) clearTimeout(searchDebounceTimer)
		searchDebounceTimer = setTimeout(() => {
			searchWorkspaceItems(workspaceSearchQuery)
		}, 300)
	}

	async function handleWorkspaceItemSelect(path: string) {
		const workspace = $workspaceStore
		if (!workspace || !onSelectWorkspaceItem) return

		try {
			if (currentView === 'scripts') {
				const script = await workspaceRunnablesSearch.getScript(path, workspace)
				const element: WorkspaceScriptElement & { deletable: boolean } = {
					type: 'workspace_script',
					path: script.path,
					title: script.path,
					summary: script.summary,
					language: script.language,
					content: script.content,
					schema: script.schema,
					deletable: true
				}
				onSelectWorkspaceItem(element)
			} else if (currentView === 'flows') {
				const flow = await workspaceRunnablesSearch.getFlow(path, workspace)
				const flowValue = JSON.stringify(flow.value, null, 2)
				const truncatedValue =
					flowValue.length > MAX_RUNNABLE_CONTENT_LENGTH
						? flowValue.slice(0, MAX_RUNNABLE_CONTENT_LENGTH) + '\n... (truncated)'
						: flowValue
				const element: WorkspaceFlowElement & { deletable: boolean } = {
					type: 'workspace_flow',
					path: flow.path,
					title: flow.path,
					summary: flow.summary,
					description: flow.description || '',
					value: truncatedValue,
					schema: flow.schema,
					deletable: true
				}
				onSelectWorkspaceItem(element)
			}
		} catch (err) {
			console.error('Error fetching workspace item', err)
		}
		currentView = 'categories'
		workspaceSearchQuery = ''
		workspaceSearchResults = []
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (isSearchableView) {
			// Navigation in workspace search view
			if (e.key === 'ArrowDown') {
				e.preventDefault()
				e.stopPropagation()
				if (workspaceSearchResults.length > 0) {
					itemSelectedIndex = (itemSelectedIndex + 1) % workspaceSearchResults.length
				}
			} else if (e.key === 'ArrowUp') {
				e.preventDefault()
				e.stopPropagation()
				if (workspaceSearchResults.length > 0) {
					itemSelectedIndex =
						(itemSelectedIndex - 1 + workspaceSearchResults.length) %
						workspaceSearchResults.length
				}
			} else if (e.key === 'Enter') {
				// Only select if not typing in the search input, or if results exist
				if (workspaceSearchResults.length > 0) {
					e.preventDefault()
					e.stopPropagation()
					const selectedItem = workspaceSearchResults[itemSelectedIndex]
					if (selectedItem) {
						handleWorkspaceItemSelect(selectedItem.path)
					}
				}
			} else if (e.key === 'Escape') {
				e.preventDefault()
				e.stopPropagation()
				handleBackClick()
			}
		} else if (stringSearch.length > 0) {
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

	// Trigger initial search when entering a searchable category
	$effect(() => {
		if (isSearchableView) {
			searchWorkspaceItems('')
		}
	})
</script>

<div
	class="flex flex-col gap-1 text-primary text-xs p-1 pr-0 min-w-24 max-h-48 overflow-y-scroll"
	onmousedown={(e) => {
		// avoids triggering onblur on the textinput and closing the tooltip
		// but allow input elements to receive focus for the search input
		if (!(e.target instanceof HTMLInputElement)) {
			e.preventDefault()
		}
	}}
	role="listbox"
	tabindex={0}
>
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
			<div class="text-center text-primary text-xs py-2">No matching context</div>
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
			<div class="text-center text-primary text-xs py-2">No available context</div>
		{/if}
	{:else if isSearchableView}
		<!-- Workspace search view (scripts/flows) -->
		<button
			class="hover:bg-surface-hover rounded-md text-left flex flex-row gap-1 items-center font-normal transition-colors mb-1"
			onclick={handleBackClick}
		>
			<ArrowLeft size={12} />
			<span class="text-xs">Go back</span>
		</button>

		<input
			bind:this={searchInputElement}
			bind:value={workspaceSearchQuery}
			oninput={handleSearchInput}
			type="text"
			placeholder="Search {currentView}..."
			class="w-full text-xs px-2 py-1 rounded-md border border-gray-200 dark:border-gray-700 bg-surface mb-1 outline-none focus:border-blue-500"
		/>

		{#if workspaceSearchLoading}
			<div class="flex items-center justify-center py-2 gap-1">
				<Loader2 size={14} class="animate-spin" />
				<span class="text-xs text-secondary">Searching...</span>
			</div>
		{:else if workspaceSearchResults.length === 0}
			<div class="text-center text-secondary text-xs py-2">
				No results found
			</div>
		{:else}
			{#each workspaceSearchResults as item, i}
				{@const isAlreadySelected = selectedContext.some(
					(c) =>
						((c.type === 'workspace_script' && currentView === 'scripts') ||
							(c.type === 'workspace_flow' && currentView === 'flows')) &&
						c.title === item.path
				)}
				<button
					class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-col font-normal transition-colors {i ===
					itemSelectedIndex
						? 'bg-surface-hover'
						: ''} {isAlreadySelected ? 'opacity-50' : ''}"
					onclick={() => {
						if (!isAlreadySelected) {
							handleWorkspaceItemSelect(item.path)
						}
					}}
					disabled={isAlreadySelected}
				>
					<div class="flex flex-row gap-1 items-center">
						{#if currentView === 'scripts'}
							<Code2 size={14} class="shrink-0" />
						{:else}
							<BarsStaggered size={14} class="shrink-0" />
						{/if}
						<span class="truncate">{item.path}</span>
					</div>
					{#if item.summary}
						<span class="truncate text-secondary pl-5">{item.summary}</span>
					{/if}
				</button>
			{/each}
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
						{element.type === 'diff' ? element.title.replace(/_/g, ' ') : element.title}
					</span>
				</button>
			{/each}
		{/if}
	{/if}
</div>
