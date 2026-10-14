import { SquareCheckBig } from 'lucide-svelte'
import type { Item } from '$lib/utils'

/**
 * Wiring for a row whose kind icon doubles as a selection control: the icon
 * swaps to a checkbox on hover, and stays one while a selection is active.
 * Distinct from `Row`'s `isSelectable`, which adds a permanent leading checkbox
 * column — this variant leaves the default row untouched until it is used.
 */
export type RowSelection = {
	/** Stable row identity; also emitted as `data-row-selection-key` so a caller
	 * can read the rendered order back from the DOM (for a shift-click range). */
	key: string
	selected: boolean
	/** Selection mode is on: every row shows its checkbox, and clicking the row
	 * toggles it instead of opening the item. */
	active: boolean
	onToggle: (e: MouseEvent | KeyboardEvent) => void
}

/**
 * The row menu's way into a selection, for the rows that offer one. The icon
 * checkbox is the fast path but only appears on hover; this is the one a user
 * can find by looking.
 */
export function selectMenuItems(rowSelection: RowSelection | undefined): Item[] {
	if (!rowSelection) return []
	return [
		{
			displayName: rowSelection.selected ? 'Deselect' : 'Select',
			icon: SquareCheckBig,
			action: (e) => rowSelection.onToggle(e)
		}
	]
}
