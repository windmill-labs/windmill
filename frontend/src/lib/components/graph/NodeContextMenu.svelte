<script lang="ts">
	import ContextMenu, { type ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'
	import { StickyNote } from 'lucide-svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		children: Snippet
		selectedNodeIds: string[]
		onCreateGroupNote?: (selectedNodeIds: string[]) => void
	}

	let { children, selectedNodeIds, onCreateGroupNote }: Props = $props()

	const menuItems: ContextMenuItem[] = $derived([
		{
			id: 'create-group-note',
			label: `Create group note (${selectedNodeIds.length} nodes)`,
			icon: StickyNote,
			disabled: selectedNodeIds.length === 0,
			onClick: () => {
				if (selectedNodeIds.length > 0) {
					onCreateGroupNote?.(selectedNodeIds)
				}
			}
		}
	])
</script>

<ContextMenu items={menuItems}>
	{@render children()}
</ContextMenu>
