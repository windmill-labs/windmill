<script lang="ts">
	import { useOnSelectionChange, useStore, type Node } from '@xyflow/svelte'
	import type { SelectionManager } from './selectionUtils.svelte'
	interface Props {
		selectionManager: SelectionManager
		clearGraphSelection: () => void
	}

	let { selectionManager, clearGraphSelection }: Props = $props()

	selectionManager.setClearGraphSelection(clearGraphSelection)

	// Get store to access selectionRect
	const store = useStore()

	// Handle selection changes from SvelteFlow
	useOnSelectionChange(({ nodes: selectedNodes, edges: _selectedEdges }) => {
		// Notes are already non-selectable, so no filtering needed
		const selectedNodeIds = selectedNodes.map((node: Node) => node.id)

		// Only select nodes if multiple nodes are selected
		// To avoid conflicting with the node-level click events
		if (selectedNodeIds.length > 0) {
			selectionManager.selectNodes(selectedNodes)
		}
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
