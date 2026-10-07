import { Move, Copy, Trash2, StickyNote, PictureInPicture2 } from 'lucide-svelte'
import { isMac, type Item } from '$lib/utils'
import type { ContextMenuItem } from '../common/contextmenu/ContextMenu.svelte'
import type { GraphEventHandlers } from './graphBuilder.svelte'
import type { NoteEditor } from './noteEditor.svelte'
import type { SelectionManager } from './selectionUtils.svelte'

/** A step's context and ellipsis menu in the editor, shared by every graph renderer. */
export function buildModuleMenuItems(opts: {
	id: string
	eventHandlers: Pick<GraphEventHandlers, 'move' | 'duplicate' | 'delete'>
	noteEditor: NoteEditor | undefined
	selectionManager: SelectionManager | undefined
	/**
	 * In modal-panel mode (sessions) step details open on double-click, or on a click on the
	 * already selected step. Surface the action in the menu too, with the gesture that works from
	 * any state as its shortcut.
	 */
	stepExploreHint: (() => boolean) | undefined
}): Item[] {
	const { id, eventHandlers, noteEditor, selectionManager, stepExploreHint } = opts
	const noteDisabled = !noteEditor || noteEditor.isNodeOnlyMemberOfGroupNote(id)
	return [
		...(stepExploreHint?.()
			? [
					{
						displayName: 'Open details',
						icon: PictureInPicture2,
						shortcut: 'Double click',
						action: () => selectionManager?.selectId(id, { openPanel: true })
					}
				]
			: []),
		...(id === 'preprocessor'
			? []
			: [
					{ displayName: 'Move', icon: Move, action: () => eventHandlers.move({ id }) },
					{ displayName: 'Duplicate', icon: Copy, action: () => eventHandlers.duplicate({ id }) }
				]),
		{
			displayName: 'Delete',
			icon: Trash2,
			type: 'delete' as const,
			shortcut: isMac() ? '⌫' : 'Del',
			action: () => eventHandlers.delete({ id }, '')
		},
		{
			displayName: 'Add note',
			icon: StickyNote,
			separatorTop: true,
			disabled: noteDisabled,
			action: () => {
				if (noteEditor && !noteDisabled) noteEditor.createGroupNote([id])
			}
		}
	]
}

/** The right-click menu's shape for the same items the ellipsis dropdown takes */
export function toContextMenuItems(items: Item[]): ContextMenuItem[] {
	return items.flatMap((item) => [
		...(item.separatorTop ? [{ id: `${item.displayName}-divider`, label: '', divider: true }] : []),
		{
			id: item.displayName,
			label: item.displayName,
			icon: item.icon,
			disabled: item.disabled,
			type: item.type,
			shortcut: item.shortcut,
			onClick: item.action as (() => void) | undefined
		}
	])
}
