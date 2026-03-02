<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { StickyNote, Move, Copy, Trash2, EllipsisVertical } from 'lucide-svelte'
	import { Button } from '../common'
	import DropdownV2 from '../DropdownV2.svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGraphContext } from './graphContext'
	import MoveHandleButton from './MoveHandleButton.svelte'
	import { tick } from 'svelte'
	import type { Item } from '$lib/utils'

	interface Props {
		selectedNodes: string[]
		allNodes: (Node & { type: string })[]
		onDeleteSelected?: () => void
		onDuplicateSelected?: () => void
		onMoveSelected?: () => void
		onCancelMove?: () => void
		canMoveSelected?: boolean
		isMoving?: boolean
		resolvedModuleIds?: string[]
	}

	let {
		selectedNodes,
		allNodes,
		onDeleteSelected,
		onDuplicateSelected,
		onMoveSelected,
		onCancelMove,
		canMoveSelected = false,
		isMoving = false,
		resolvedModuleIds = []
	}: Props = $props()

	let resolvedCount = $derived(resolvedModuleIds.length)

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get Graph context for clearFlowSelection function and moveManager
	const graphContext = getGraphContext()
	const moveManager = graphContext?.moveManager

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

	let menuItems: Item[] = $derived([
		{
			displayName: 'Move',
			icon: Move,
			action: () => onMoveSelected?.(),
			disabled: !canMoveSelected
		},
		{
			displayName: 'Duplicate',
			icon: Copy,
			action: () => onDuplicateSelected?.()
		},
		{
			displayName: `Delete (${resolvedCount})`,
			icon: Trash2,
			type: 'delete',
			shortcut: '⌫',
			action: () => onDeleteSelected?.()
		},
		...(noteEditorContext?.noteEditor
			? [
					{
						displayName: 'Add note',
						icon: StickyNote,
						separatorTop: true,
						action: handleAddGroupNote
					}
				]
			: [])
	])

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
			<div
				class="absolute -top-4 -right-1 z-20 flex gap-2 items-center"
				style="pointer-events: auto;"
			>
				{#if isMoving}
					<Button variant="accent" onClick={() => onCancelMove?.()} size="xs" destructive
						>Cancel move</Button
					>
				{:else if resolvedCount > 0}
					{#if canMoveSelected && moveManager && resolvedModuleIds.length > 0}
						<div class="">
							<MoveHandleButton
								{moveManager}
								moduleId={resolvedModuleIds[0]}
								selectedIds={resolvedModuleIds}
								onClickMove={() => onMoveSelected?.()}
							/>
						</div>
					{/if}
					<DropdownV2 items={menuItems} size="sm" placement="right-start">
						{#snippet buttonReplacement()}
							<button
								class="center-center p-1 rounded-md shadow-md bg-surface text-secondary hover:bg-surface-tertiary"
								title="Actions"
							>
								<EllipsisVertical size={12} />
							</button>
						{/snippet}
					</DropdownV2>
				{/if}
			</div>
		</div>
	</ViewportPortal>
{/if}
