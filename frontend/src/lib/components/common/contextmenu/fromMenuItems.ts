import type { Item } from '$lib/utils'
import type { ContextMenuItem } from './ContextMenu.svelte'

/** A dropdown's items as the same entries of a right-click menu. */
export function contextMenuItemsFromMenu(items: Item[]): ContextMenuItem[] {
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
