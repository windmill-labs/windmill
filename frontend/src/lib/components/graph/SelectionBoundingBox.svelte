<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { StickyNote, Group } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGroupEditorContext } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { tick } from 'svelte'

	interface Props {
		selectedNodes: string[]
		allNodes: (Node & { type: string })[]
	}

	let { selectedNodes, allNodes }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get GroupEditor context for group creation
	const groupEditorContext = getGroupEditorContext()
	// Get Graph context for clearFlowSelection function
	const graphContext = getGraphContext()

	function handleAddGroupNote() {
		if (selectedNodes.length > 0 && noteEditorContext?.noteEditor && graphContext) {
			noteEditorContext.noteEditor.createGroupNote(selectedNodes)

			tick().then(() => {
				graphContext?.clearFlowSelection?.()
				graphContext?.selectionManager.clearSelection()
			})
		}
	}

	function handleAddGroup() {
		if (selectedNodes.length > 0 && groupEditorContext?.groupEditor && graphContext) {
			groupEditorContext.groupEditor.createGroup(selectedNodes)

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

		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(selectedNodes, allNodes)

		const padding = 4

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
			{#if noteEditorContext?.noteEditor || groupEditorContext?.groupEditor}
				<div class="absolute -top-4 -right-1 z-20 flex gap-1" style="pointer-events: auto;">
					{#if groupEditorContext?.groupEditor}
						<Button
							unifiedSize="sm"
							variant="accent"
							title="Create group ({selectedNodes.length} nodes)"
							onclick={handleAddGroup}
							startIcon={{ icon: Group }}
						>
							Create group
						</Button>
					{/if}
					{#if noteEditorContext?.noteEditor}
						<Button
							unifiedSize="sm"
							variant="accent"
							title="Create group note ({selectedNodes.length} nodes)"
							onclick={handleAddGroupNote}
							startIcon={{ icon: StickyNote }}
						>
							Create group note
						</Button>
					{/if}
				</div>
			{/if}
		</div>
	</ViewportPortal>
{/if}
