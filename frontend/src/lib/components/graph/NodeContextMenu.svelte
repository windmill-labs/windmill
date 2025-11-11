<script lang="ts">
	import ContextMenu, { type ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'
	import { StickyNote } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'

	interface Props {
		children: Snippet
		selectedNodeIds: string[]
	}

	let { children, selectedNodeIds }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()

	const menuItems: ContextMenuItem[] = $derived([
		{
			id: 'create-group-note',
			label: `Create group note (${selectedNodeIds.length} nodes)`,
			icon: StickyNote,
			disabled: selectedNodeIds.length === 0 || !noteEditorContext?.noteEditor,
			onClick: () => {
				if (selectedNodeIds.length > 0 && noteEditorContext?.noteEditor) {
					noteEditorContext.noteEditor.createGroupNote(selectedNodeIds, 'Group Note')
				}
			}
		}
	])
</script>

{#if noteEditorContext?.noteEditor}
	<ContextMenu items={menuItems}>
		{@render children()}
	</ContextMenu>
{/if}
