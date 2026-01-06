<script lang="ts">
	import { Building2, Search, ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte'
	import { SvelteMap } from 'svelte/reactivity'
	import WorkspaceCard from './WorkspaceCard.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import type { UserWorkspace } from '$lib/stores'
	import { Button } from '../common'

	interface ExtendedWorkspace extends UserWorkspace {
		_children?: ExtendedWorkspace[]
		marked?: string
	}

	interface Props {
		workspaces: UserWorkspace[]
		onEnterWorkspace: (workspaceId: string) => Promise<void>
		onUnarchive?: (workspaceId: string) => Promise<void>
		searchFilter?: string
		showControls?: boolean
		onExpandCollapseAll?: () => void
		allExpanded?: boolean
		hasForks?: boolean
	}

	let {
		workspaces,
		onEnterWorkspace,
		onUnarchive,
		searchFilter = $bindable(''),
		showControls = true,
		onExpandCollapseAll = $bindable(),
		allExpanded = $bindable(false),
		hasForks = $bindable(false)
	}: Props = $props()

	// State for manually toggled expansion status
	let manualExpansionStates = $state<Record<string, boolean>>({})
	let filteredWorkspaces: (UserWorkspace & { marked?: string })[] | undefined = $state()

	// Computed expansion states that include auto-expansion for search results
	let expansionStates = $derived.by(() => {
		if (!searchFilter || !filteredWorkspaces || !workspaces) {
			return manualExpansionStates
		}

		const matchedWorkspaceIds = new Set(filteredWorkspaces.map((w) => w.id))
		const autoExpanded: Record<string, boolean> = {}

		// Build children map for descendant checking
		const childrenMap = new SvelteMap<string, string[]>()
		workspaces.forEach((workspace) => {
			if (workspace.parent_workspace_id) {
				if (!childrenMap.has(workspace.parent_workspace_id)) {
					childrenMap.set(workspace.parent_workspace_id, [])
				}
				childrenMap.get(workspace.parent_workspace_id)!.push(workspace.id)
			}
		})

		// Function to check if workspace has matching descendants (not itself)
		function hasMatchingDescendant(workspaceId: string): boolean {
			const children = childrenMap.get(workspaceId) || []
			return children.some(
				(childId) => matchedWorkspaceIds.has(childId) || hasMatchingDescendant(childId)
			)
		}

		// Auto-expand workspaces only if they have matching descendants
		// (not if the workspace itself matches - user can manually expand to see children)
		workspaces.forEach((workspace) => {
			if (hasMatchingDescendant(workspace.id)) {
				autoExpanded[workspace.id] = true
			}
		})

		// Combine manual and auto-expanded states
		return { ...manualExpansionStates, ...autoExpanded }
	})

	// Build nested hierarchy correctly - always use full workspace list for hierarchy
	let rootWorkspaces = $derived.by(() => {
		if (!workspaces) return []

		// Create a map of children for each parent workspace using ALL workspaces
		const childrenMap = new SvelteMap<string, ExtendedWorkspace[]>()

		// Build children mapping - this correctly handles nested relationships
		workspaces.forEach((workspace) => {
			if (workspace.parent_workspace_id) {
				if (!childrenMap.has(workspace.parent_workspace_id)) {
					childrenMap.set(workspace.parent_workspace_id, [])
				}
				// Find marked version from filtered results if available
				const filteredWorkspace = filteredWorkspaces?.find((fw) => fw.id === workspace.id)
				const extendedWorkspace = {
					...workspace,
					marked: filteredWorkspace?.marked
				} as ExtendedWorkspace
				childrenMap.get(workspace.parent_workspace_id)!.push(extendedWorkspace)
			}
		})

		// Get IDs of workspaces that match the search
		const matchedWorkspaceIds = new Set(filteredWorkspaces?.map((w) => w.id) || [])

		// Function to check if a workspace or its descendants match the search
		function hasMatchingDescendant(workspaceId: string): boolean {
			if (matchedWorkspaceIds.has(workspaceId)) return true
			const children = childrenMap.get(workspaceId) || []
			return children.some((child) => hasMatchingDescendant(child.id))
		}

		// Recursive function to build full nested hierarchy
		// parentMatched: if true, include all children regardless of search match
		function buildWorkspaceWithChildren(
			workspace: UserWorkspace,
			parentMatched: boolean = false
		): ExtendedWorkspace {
			const directChildren = childrenMap.get(workspace.id) || []
			const thisWorkspaceMatches = matchedWorkspaceIds.has(workspace.id)

			// If this workspace or a parent matches, show all children
			// Otherwise, only show children that match or have matching descendants
			const visibleChildren =
				searchFilter && !parentMatched && !thisWorkspaceMatches
					? directChildren.filter((child) => hasMatchingDescendant(child.id))
					: directChildren

			const childrenWithNestedStructure = visibleChildren.map((child) =>
				buildWorkspaceWithChildren(child, parentMatched || thisWorkspaceMatches)
			)

			// Find marked version from filtered results if available
			const filteredWorkspace = filteredWorkspaces?.find((fw) => fw.id === workspace.id)

			return {
				...workspace,
				marked: filteredWorkspace?.marked,
				_children: childrenWithNestedStructure
			}
		}

		// Return only root workspaces - filter based on search if active
		const rootCandidates = workspaces.filter(
			(workspace) =>
				!workspace.parent_workspace_id ||
				!workspaces.find((w) => w.id === workspace.parent_workspace_id)
		)

		const visibleRoots = searchFilter
			? rootCandidates.filter((workspace) => hasMatchingDescendant(workspace.id))
			: rootCandidates

		return visibleRoots
			.map((workspace) => buildWorkspaceWithChildren(workspace))
			.sort((a, b) => a.name.localeCompare(b.name))
	})

	function handleToggleExpand(workspaceId: string) {
		const currentState = expansionStates[workspaceId] ?? false
		manualExpansionStates = { ...manualExpansionStates, [workspaceId]: !currentState }
	}

	// Get IDs of workspaces that have children (can be expanded)
	let workspacesWithChildren = $derived.by(() => {
		if (!workspaces) return []
		const parentIds = new Set(
			workspaces.filter((w) => w.parent_workspace_id).map((w) => w.parent_workspace_id)
		)
		return workspaces.filter((w) => parentIds.has(w.id)).map((w) => w.id)
	})

	// Check if all expandable workspaces are currently expanded
	let allExpandedInternal = $derived(
		workspacesWithChildren.length > 0 &&
			workspacesWithChildren.every((id) => expansionStates[id] === true)
	)

	// Sync internal state to bindable props
	$effect(() => {
		allExpanded = allExpandedInternal
	})

	$effect(() => {
		hasForks = workspacesWithChildren.length > 0
	})

	function handleExpandCollapseAll() {
		const newState = !allExpandedInternal
		const newExpansionStates: Record<string, boolean> = {}
		workspacesWithChildren.forEach((id) => {
			newExpansionStates[id] = newState
		})
		manualExpansionStates = newExpansionStates
	}

	// Expose the function via bindable prop
	$effect(() => {
		onExpandCollapseAll = handleExpandCollapseAll
	})
</script>

<!-- Search Items Component for fuzzy search with highlighting -->
<SearchItems
	filter={searchFilter}
	items={workspaces}
	bind:filteredItems={filteredWorkspaces}
	f={(workspace) => workspace.name + ' (' + workspace.id + ')'}
/>

<div class="space-y-4">
	<!-- Search Input and Expand/Collapse All (only show if more than one workspace and showControls is true) -->
	{#if showControls && workspaces.length > 1}
		<div class="flex gap-2 items-center">
			<div class="relative text-primary flex-1">
				<TextInput
					inputProps={{
						placeholder: 'Search workspaces...'
					}}
					size="md"
					bind:value={searchFilter}
					class="!pr-10"
				/>
				<Search size={16} class="text-secondary absolute right-2 top-0 mt-2" />
			</div>
			{#if workspacesWithChildren.length > 0}
				<Button
					onClick={handleExpandCollapseAll}
					title={allExpandedInternal ? 'Collapse all' : 'Expand all'}
					startIcon={{ icon: allExpandedInternal ? ChevronsDownUp : ChevronsUpDown }}
					unifiedSize="md"
					variant="default"
				>
					{#if allExpandedInternal}
						<span>Collapse</span>
					{:else}
						<span>Expand</span>
					{/if}
				</Button>
			{/if}
		</div>
	{/if}

	<!-- Workspace Tree -->
	<div class="space-y-2">
		{#each rootWorkspaces as workspace (workspace.id)}
			<WorkspaceCard
				{workspace}
				children={workspace._children || []}
				isExpanded={expansionStates[workspace.id] ?? false}
				{expansionStates}
				{onEnterWorkspace}
				{onUnarchive}
				onToggleExpand={handleToggleExpand}
			/>
		{/each}

		{#if rootWorkspaces.length === 0}
			<div class="text-center py-8">
				<Building2 size={48} class="text-secondary mx-auto mb-3" />
				<p class="text-sm text-secondary">
					{searchFilter ? 'No workspaces match your search' : 'No workspaces available'}
				</p>
			</div>
		{/if}
	</div>
</div>
