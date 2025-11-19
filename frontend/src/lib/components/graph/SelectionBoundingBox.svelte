<script lang="ts">
	import type { Node } from '@xyflow/svelte'
	import { useSvelteFlow } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { StickyNote } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { tick } from 'svelte'

	interface Props {
		allNodes: Node[]
	}

	let { allNodes }: Props = $props()

	const { flowToScreenPosition } = useSvelteFlow()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get Graph context for clearFlowSelection function
	const graphContext = getGraphContext()

	const selectedNodes = $derived(allNodes.filter((node) => node.selected).map((node) => node.id))

	function handleAddGroupNote() {
		if (selectedNodes.length > 0 && noteEditorContext?.noteEditor && graphContext) {
			// Create the group note first
			noteEditorContext.noteEditor.createGroupNote(selectedNodes)

			// Wait for next tick to ensure DOM updates
			tick().then(() => {
				graphContext?.clearFlowSelection?.()
				graphContext?.selectionManager.clearSelection()
			})
		}
	}

	let bounds = $derived(() => {
		if (selectedNodes.length === 0) {
			return null
		}

		// Calculate flow coordinates bounds, accounting for CSS offset and expanded subflows
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(selectedNodes, allNodes)

		// Add padding in flow coordinates

		const padding = 4
		const flowBounds = {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
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
		class={'absolute cursor-pointer bg-surface-selected/40 rounded-md pointer-events-none'}
		style="
			left: {currentBounds.x}px;
			top: {currentBounds.y}px;
			width: {currentBounds.width}px;
			height: {currentBounds.height}px;
			z-index: 10;
		"
	>
		<!-- Add Group Note Button positioned in top-right corner -->
		{#if noteEditorContext?.noteEditor}
			<div class="absolute -top-4 -right-1 z-20" style="pointer-events: auto;">
				<Button
					unifiedSize="sm"
					variant="accent"
					title="Create group note ({selectedNodes.length} nodes)"
					onclick={handleAddGroupNote}
					startIcon={{ icon: StickyNote }}
				>
					Create group note ({selectedNodes.length} nodes)
				</Button>
			</div>
		{/if}
	</div>
{/if}
