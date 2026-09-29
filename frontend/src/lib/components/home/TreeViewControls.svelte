<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte'
	import type { TreeViewState } from './treeViewState.svelte'

	interface Props {
		tree: TreeViewState
		// The tree is showing every node open (a filter is active), so Expand/Collapse all
		// would have no effect.
		forceExpanded?: boolean
		class?: string
	}

	let { tree, forceExpanded = false, class: clazz = '' }: Props = $props()
</script>

<div class="flex items-center gap-2 {clazz}">
	<Toggle size="xs" bind:checked={tree.treeView} options={{ right: 'Tree view' }} />
	{#if tree.treeView && !forceExpanded}
		<Button
			unifiedSize="sm"
			variant="subtle"
			on:click={() => (tree.collapseAll = !tree.collapseAll)}
			startIcon={{ icon: tree.collapseAll ? ChevronsUpDown : ChevronsDownUp }}
		>
			{tree.collapseAll ? 'Expand all' : 'Collapse all'}
		</Button>
	{/if}
</div>
