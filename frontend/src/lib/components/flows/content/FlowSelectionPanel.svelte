<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'
	import type { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { getNoteEditorContext } from '$lib/components/graph/noteEditor.svelte'
	import { StickyNote, Move, Copy, Trash2 } from 'lucide-svelte'
	import type { Item } from '$lib/utils'

	interface Props {
		selectionManager: SelectionManager
		noEditor: boolean
		onDeleteSelected?: () => void
		onDuplicateSelected?: () => void
		onMoveSelected?: () => void
		canMoveSelected?: boolean
		resolvedCount?: number
	}
	let {
		selectionManager,
		noEditor,
		onDeleteSelected,
		onDuplicateSelected,
		onMoveSelected,
		canMoveSelected = false,
		resolvedCount = 0
	}: Props = $props()

	const noteEditorContext = getNoteEditorContext()

	function addGroupNote() {
		if (selectionManager.selectedIds.length > 0 && noteEditorContext?.noteEditor) {
			// Create the group note
			noteEditorContext.noteEditor.createGroupNote(selectionManager.selectedIds)
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
			action: () => onDeleteSelected?.()
		}
	])
</script>

<FlowCard {noEditor} title="Multiple Selection">
	{#snippet action()}
		<div class="flex gap-1 items-center">
			<Button
				onClick={addGroupNote}
				disabled={!noteEditorContext?.noteEditor || selectionManager.selectedIds.length === 0}
				startIcon={{ icon: StickyNote }}
			>
				Create group note
			</Button>
			{#if resolvedCount > 0}
				<DropdownV2 items={menuItems} />
			{/if}
		</div>
	{/snippet}
	<div class="px-4">
		<div class="space-y-2 mb-4">
			{#each selectionManager.selectedIds as nodeId}
				<div class="text-sm px-2 py-1 bg-surface rounded border">
					{nodeId}
				</div>
			{/each}
		</div>
	</div>
</FlowCard>
