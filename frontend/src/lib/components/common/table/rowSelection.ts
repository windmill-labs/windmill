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
