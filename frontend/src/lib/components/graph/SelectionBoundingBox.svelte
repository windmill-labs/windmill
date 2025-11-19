<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { StickyNote } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { tick } from 'svelte'

	interface Props {
		selectedNodes: string[]
		allNodes: (Node & { type: string })[]
	}

	let { selectedNodes, allNodes }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get Graph context for clearFlowSelection function
	const graphContext = getGraphContext()

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

		// Return flow coordinates directly - ViewportPortal handles transformation
		return {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
		}
	})
</script>

{#if bounds() && selectedNodes.length > 1}
	{@const currentBounds = bounds()!}
	<ViewportPortal target="front">
		<div
			class={'absolute cursor-pointer bg-surface-selected/40 rounded-md pointer-events-none'}
			style:transform="translate({currentBounds.x}px, {currentBounds.y}px)"
			style:width="{currentBounds.width}px"
			style:height="{currentBounds.height}px"
			style:z-index="10"
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
	</ViewportPortal>
{/if}
