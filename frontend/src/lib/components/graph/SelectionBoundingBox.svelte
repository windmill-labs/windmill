<script lang="ts">
	import type { Node } from '@xyflow/svelte'
	import { NODE } from './util'

	interface Props {
		selectedNodes: Node[]
	}

	let { selectedNodes }: Props = $props()

	let bounds = $derived(() => {
		if (selectedNodes.length === 0) {
			return null
		}

		let minX = Infinity
		let maxX = -Infinity
		let minY = Infinity
		let maxY = -Infinity

		selectedNodes.forEach(node => {
			minX = Math.min(minX, node.position.x)
			maxX = Math.max(maxX, node.position.x + NODE.width)
			minY = Math.min(minY, node.position.y)
			maxY = Math.max(maxY, node.position.y + NODE.height)
		})

		return {
			x: minX - 10, // Add padding
			y: minY - 10,
			width: maxX - minX + 20,
			height: maxY - minY + 20
		}
	})
</script>

{#if bounds() && selectedNodes.length > 1}
	{@const currentBounds = bounds()!}
	<div
		class="absolute pointer-events-none border-2 border-dashed border-accent bg-accent/5 rounded"
		style="
			left: {currentBounds.x}px;
			top: {currentBounds.y}px;
			width: {currentBounds.width}px;
			height: {currentBounds.height}px;
			z-index: -1;
		"
	>
		<div class="absolute -top-6 left-0 text-xs text-accent font-medium bg-surface px-2 py-1 rounded shadow">
			{selectedNodes.length} nodes selected
		</div>
	</div>
{/if}