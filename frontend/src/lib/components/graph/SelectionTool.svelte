<script lang="ts">
	import { useOnSelectionChange, useStore, type Node } from '@xyflow/svelte'

	interface Props {
		nodes: any[]
		modules?: any[]
		selectionManager: any
	}

	let { nodes, modules, selectionManager }: Props = $props()

	// Get store to access selectionRect
	const store = useStore()

	// Handle selection changes from SvelteFlow
	useOnSelectionChange(({ nodes: selectedNodes, edges: _selectedEdges }) => {
		console.log('dbg useOnSelectionChange', selectedNodes, _selectedEdges)
		// Notes are already non-selectable, so no filtering needed
		const selectedNodeIds = selectedNodes.map((node: Node) => node.id)

		if (selectedNodeIds.length > 0) {
			selectionManager.selectNodes(selectedNodeIds, false, modules, nodes)
		} else if (selectedNodes.length === 0) {
			// Clear selection when SvelteFlow selection is cleared
			selectionManager.clearSelection()
		}
	})

	// Compute selection box bounds
	let selectionBoxBounds = $derived(() => {
		const rect = store.selectionRect
		if (!rect) {
			return null
		}

		// selectionRect is already in the correct coordinate system relative to the flow container
		// Just return it directly
		return {
			x: rect.x,
			y: rect.y,
			width: rect.width,
			height: rect.height
		}
	})
</script>

<!-- Render custom selection box during drag selection -->
{#if selectionBoxBounds()}
	{@const bounds = selectionBoxBounds()!}
	<div
		class="absolute rounded cursor-pointer bg-surface-selected/30 border border-accent/30 pointer-events-none"
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
