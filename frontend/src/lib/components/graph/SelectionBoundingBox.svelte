<script lang="ts">
	import type { Node } from '@xyflow/svelte'
	import { useSvelteFlow } from '@xyflow/svelte'
	import { NODE } from './util'

	interface Props {
		selectedNodes: Node[]
	}

	let { selectedNodes }: Props = $props()

	const { flowToScreenPosition } = useSvelteFlow()

	let bounds = $derived(() => {
		if (selectedNodes.length === 0) {
			return null
		}

		// Calculate flow coordinates bounds
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

		// Add padding in flow coordinates
		const flowBounds = {
			x: minX - 10,
			y: minY - 10,
			width: maxX - minX + 20,
			height: maxY - minY + 20
		}

		// Convert to screen coordinates relative to the SvelteFlow container
		const topLeftScreen = flowToScreenPosition({ x: flowBounds.x, y: flowBounds.y })
		const bottomRightScreen = flowToScreenPosition({
			x: flowBounds.x + flowBounds.width,
			y: flowBounds.y + flowBounds.height
		})

		// Get the SvelteFlow container's position to make coordinates relative
		const flowElement = document.querySelector('.svelte-flow')
		const flowRect = flowElement?.getBoundingClientRect()

		if (!flowRect) {
			return {
				x: topLeftScreen.x,
				y: topLeftScreen.y,
				width: bottomRightScreen.x - topLeftScreen.x,
				height: bottomRightScreen.y - topLeftScreen.y
			}
		}

		return {
			x: topLeftScreen.x - flowRect.left,
			y: topLeftScreen.y - flowRect.top,
			width: bottomRightScreen.x - topLeftScreen.x,
			height: bottomRightScreen.y - topLeftScreen.y
		}
	})
</script>

{#if bounds() && selectedNodes.length > 1}
	{@const currentBounds = bounds()!}
	<div
		class="absolute border-2 border-dashed border-accent bg-accent/5 rounded cursor-pointer"
		style="
			left: {currentBounds.x}px;
			top: {currentBounds.y}px;
			width: {currentBounds.width}px;
			height: {currentBounds.height}px;
			z-index: 10;
		"
	>
		<div class="absolute -top-6 left-0 text-xs text-accent font-medium bg-surface px-2 py-1 rounded shadow pointer-events-none">
			{selectedNodes.length} nodes selected
		</div>
	</div>
{/if}