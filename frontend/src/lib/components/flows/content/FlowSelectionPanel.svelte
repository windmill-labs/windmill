<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'
	import type { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { getGroupEditorContext } from '$lib/components/graph/groupEditor.svelte'
	import { Group, Move, Copy, Trash2 } from 'lucide-svelte'
	import type { Item } from '$lib/utils'

	interface Props {
		selectionManager: SelectionManager
		noEditor: boolean
		onDeleteSelected?: () => void
		onDuplicateSelected?: () => void
		onMoveSelected?: () => void
		onCreateGroup?: () => void
		canMoveSelected?: boolean
		resolvedCount?: number
	}
	let {
		selectionManager,
		noEditor,
		onDeleteSelected,
		onDuplicateSelected,
		onMoveSelected,
		onCreateGroup,
		canMoveSelected = false,
		resolvedCount = 0
	}: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	let canCreateGroup = $derived(groupEditorContext?.canCreateGroup.val ?? false)

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
				onClick={() => onCreateGroup?.()}
				disabled={!canCreateGroup}
				startIcon={{ icon: Group }}
			>
				Create group
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
