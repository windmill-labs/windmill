import { SquareCheckBig } from 'lucide-svelte'
import type { Item } from '$lib/utils'

/**
 * Wiring for a row that can join a multi-selection. The control lives in a
 * leading gutter the row reserves unconditionally — empty until the row is
 * hovered — so the kind icon keeps its place and starting a selection never
 * reflows the list. Distinct from `Row`'s `isSelectable`, which shows its
 * checkbox at all times.
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
 * The row menu's way into a selection, for the rows that offer one. The gutter
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
