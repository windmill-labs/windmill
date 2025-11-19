<script lang="ts">
	import { useOnSelectionChange, useStore } from '@xyflow/svelte'
	import type { SelectionManager } from './selectionUtils.svelte'
	interface Props {
		selectionManager: SelectionManager
	}

	let { selectionManager }: Props = $props()

	// Get store to access selectionRect
	const store = useStore()

	// Handle selection changes from SvelteFlow
	useOnSelectionChange(({ nodes: selectedNodes, edges: _selectedEdges }) => {
		selectionManager.selectedNodesInGraph = selectedNodes
	})
</script>

<!-- Render custom selection box during drag selection -->
{#if store.selectionRect}
	{@const bounds = store.selectionRect!}
	<div
		class="absolute rounded cursor-pointer bg-surface-selected/30 pointer-events-none"
		style="
			left: {bounds.x}px;
			top: {bounds.y}px;
			width: {bounds.width}px;
			height: {bounds.height}px;
			z-index: 10;
		"
	>
	</div>
{/if}
