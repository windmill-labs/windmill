<script lang="ts">
	import { Building2 } from 'lucide-svelte'
	import { SvelteMap } from 'svelte/reactivity'
	import WorkspaceCard from './WorkspaceCard.svelte'
	import type { UserWorkspace } from '$lib/stores'

	interface ExtendedWorkspace extends UserWorkspace {
		_children?: ExtendedWorkspace[]
	}

	interface Props {
		workspaces: UserWorkspace[]
		onEnterWorkspace: (workspaceId: string) => Promise<void>
		onUnarchive?: (workspaceId: string) => Promise<void>
	}

	let { workspaces, onEnterWorkspace, onUnarchive }: Props = $props()

	// State for each parent workspace's expansion status
	let expansionStates = $state<Record<string, boolean>>({})

	// Build nested hierarchy correctly (this was the issue before)
	let rootWorkspaces = $derived.by(() => {
		// Create a map of children for each parent workspace
		const childrenMap = new SvelteMap<string, UserWorkspace[]>()

		// Build children mapping - this correctly handles nested relationships
		workspaces.forEach(workspace => {
			if (workspace.parent_workspace_id) {
				if (!childrenMap.has(workspace.parent_workspace_id)) {
					childrenMap.set(workspace.parent_workspace_id, [])
				}
				childrenMap.get(workspace.parent_workspace_id)!.push(workspace)
			}
		})

		// Recursive function to build full nested hierarchy
		function buildWorkspaceWithChildren(workspace: UserWorkspace): ExtendedWorkspace {
			const directChildren = childrenMap.get(workspace.id) || []
			const childrenWithNestedStructure = directChildren.map(child => buildWorkspaceWithChildren(child))

			return {
				...workspace,
				_children: childrenWithNestedStructure
			}
		}

		// Return only root workspaces (those without parents or whose parents aren't in the list)
		return workspaces
			.filter(workspace =>
				!workspace.parent_workspace_id ||
				!workspaces.find(w => w.id === workspace.parent_workspace_id)
			)
			.map(workspace => buildWorkspaceWithChildren(workspace))
			.sort((a, b) => a.name.localeCompare(b.name))
	})

	function handleToggleExpand(workspaceId: string) {
		const currentState = expansionStates[workspaceId] ?? false
		expansionStates = { ...expansionStates, [workspaceId]: !currentState }
	}
</script>

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
			<p class="text-sm text-secondary">No workspaces available</p>
		</div>
	{/if}
</div>
