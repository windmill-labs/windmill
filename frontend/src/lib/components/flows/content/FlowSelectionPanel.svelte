<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'
	import type { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { Button } from '$lib/components/common'
	import { getNoteEditorContext } from '$lib/components/graph/noteEditor.svelte'

	interface Props {
		selectionManager: SelectionManager
		noEditor: boolean
	}
	let { selectionManager, noEditor }: Props = $props()

	const noteEditorContext = getNoteEditorContext()

	function addGroupNote() {
		if (selectionManager.selectedIds.length > 0 && noteEditorContext?.noteEditor) {
			// Create the group note
			noteEditorContext.noteEditor.createGroupNote(selectionManager.selectedIds)
		}
	}
</script>

<FlowCard {noEditor} title="Multiple Selection">
	<div class="px-4">
		<p class="text-xs text-secondary mb-4">{selectionManager.selectedIds.length} nodes selected</p>
		<div class="space-y-2 mb-4">
			{#each selectionManager.selectedIds as nodeId}
				<div class="text-sm px-2 py-1 bg-surface rounded border">
					{nodeId}
				</div>
			{/each}
		</div>
		<Button
			onClick={addGroupNote}
			disabled={!noteEditorContext?.noteEditor || selectionManager.selectedIds.length === 0}
		>
			Add Group Note
		</Button>
	</div>
</FlowCard>
