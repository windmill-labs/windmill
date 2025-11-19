<script lang="ts">
	import FlowCard from '../common/FlowCard.svelte'
	import type { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { Button } from '$lib/components/common'
	import { getNoteEditorContext } from '$lib/components/graph/noteEditor.svelte'
	import { StickyNote } from 'lucide-svelte'

	interface Props {
		selectionManager: SelectionManager
		noEditor: boolean
	}
	let { selectionManager, noEditor }: Props = $props()

	const noteEditorContext = getNoteEditorContext()

	const selectedNodes = $derived(selectionManager.selectedNodesInGraph.map((node) => node.id))

	function addGroupNote() {
		if (selectedNodes.length > 0 && noteEditorContext?.noteEditor) {
			// Create the group note
			noteEditorContext.noteEditor.createGroupNote(selectedNodes)
		}
	}
</script>

<FlowCard {noEditor} title="Multiple Selection">
	{#snippet action()}
		<Button
			onClick={addGroupNote}
			disabled={!noteEditorContext?.noteEditor || selectedNodes.length === 0}
			startIcon={{ icon: StickyNote }}
		>
			Create group note
		</Button>
	{/snippet}
	<div class="px-4">
		<p class="text-xs text-secondary mb-4">{selectedNodes.length} nodes selected</p>
		<div class="space-y-2 mb-4">
			{#each selectedNodes as nodeId}
				<div class="text-sm px-2 py-1 bg-surface rounded border">
					{nodeId}
				</div>
			{/each}
		</div>
	</div>
</FlowCard>
