<script lang="ts">
	import ContextMenu, { type ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'
	import { StickyNote } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { tick } from 'svelte'

	interface Props {
		children: Snippet
		selectedNodeIds: string[]
	}

	let { children, selectedNodeIds }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get Graph context for clearFlowSelection function
	const graphContext = getGraphContext()

	const menuItems: ContextMenuItem[] = $derived([
		{
			id: 'create-group-note',
			label: `Create group note (${selectedNodeIds.length} nodes)`,
			icon: StickyNote,
			disabled: selectedNodeIds.length === 0 || !noteEditorContext?.noteEditor,
			onClick: () => {
				if (selectedNodeIds.length > 0 && noteEditorContext?.noteEditor && graphContext) {
					// Create the group note first
					noteEditorContext.noteEditor.createGroupNote(selectedNodeIds)

					// Wait for next tick to ensure DOM updates
					tick().then(() => {
						graphContext?.clearFlowSelection?.()
						graphContext?.selectionManager.selectId(selectedNodeIds[0])
					})
				}
			}
		}
	])
</script>

{#if noteEditorContext?.noteEditor && selectedNodeIds.length > 1}
	<ContextMenu items={menuItems}>
		{@render children()}
	</ContextMenu>
{/if}
