<script lang="ts">
	import ContextMenu, { type ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'
	import { StickyNote, Group } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { getGroupEditorContext } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { tick } from 'svelte'

	interface Props {
		children: Snippet
		selectedNodeIds: string[]
	}

	let { children, selectedNodeIds }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	// Get GroupEditor context for group creation
	const groupEditorContext = getGroupEditorContext()
	// Get Graph context for clearFlowSelection function
	const graphContext = getGraphContext()

	const menuItems: ContextMenuItem[] = $derived([
		{
			id: 'create-group',
			label: `Create group (${selectedNodeIds.length} nodes)`,
			icon: Group,
			disabled: selectedNodeIds.length === 0 || !groupEditorContext?.groupEditor,
			onClick: () => {
				if (selectedNodeIds.length > 0 && groupEditorContext?.groupEditor && graphContext) {
					groupEditorContext.groupEditor.createGroup(selectedNodeIds)

					tick().then(() => {
						graphContext?.clearFlowSelection?.()
						graphContext?.selectionManager.selectId(selectedNodeIds[0])
					})
				}
			}
		},
		{
			id: 'create-group-note',
			label: `Create group note (${selectedNodeIds.length} nodes)`,
			icon: StickyNote,
			disabled: selectedNodeIds.length === 0 || !noteEditorContext?.noteEditor,
			onClick: () => {
				if (selectedNodeIds.length > 0 && noteEditorContext?.noteEditor && graphContext) {
					noteEditorContext.noteEditor.createGroupNote(selectedNodeIds)

					tick().then(() => {
						graphContext?.clearFlowSelection?.()
						graphContext?.selectionManager.selectId(selectedNodeIds[0])
					})
				}
			}
		}
	])
</script>

{#if (noteEditorContext?.noteEditor || groupEditorContext?.groupEditor) && selectedNodeIds.length > 1}
	<ContextMenu items={menuItems}>
		{@render children()}
	</ContextMenu>
{/if}
