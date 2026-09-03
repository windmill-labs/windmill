import { untrack } from 'svelte'

/**
 * The highlighted row of a searchable list: the one the arrow keys move and Enter
 * activates, rendered by passing `highlighted` to `ListRow`.
 *
 * Pairs with a search field above the list — the arrows and Enter are answered while
 * focus stays in it, so a query and a choice are one uninterrupted sequence.
 */
export function useListHighlight(opts: {
	/** How many rows the list holds right now. */
	count: () => number
	/** The DOM id of the row at this index — the same `id` given to its `ListRow`. */
	rowId: (index: number) => string
	/** Where the highlight belongs when the list changes underneath it: the top hit while
	 * a search is on, and typically -1 (nothing lit) when it is not. */
	restingIndex: () => number
	/** Open the row at this index. */
	onActivate: (index: number) => void
	/** Ids of the elements whose Enter also activates the highlighted row — the search
	 * field. A focused row activates itself, so it is not one of these. */
	activateEnterFrom?: string[]
}) {
	let index = $state(-1)
	// Scrolling rows under a resting pointer makes the browser fire `mouseenter` on each
	// one, which would drag the highlight back under the cursor as the arrow keys move it.
	// Only a real pointer move hands the highlight back to the mouse.
	let pointerOwns = $state(true)

	// Filtering reshuffles the rows under the highlight, so it goes back where the caller
	// says it belongs rather than staying on a position that now means another row.
	$effect(() => {
		opts.count()
		const resting = opts.restingIndex()
		untrack(() => (index = resting))
	})

	function move(delta: number) {
		const count = opts.count()
		if (count === 0) return
		pointerOwns = false
		// Rows are tabbable, so focus can sit on one. Enter then activates whatever is
		// focused, which has to stay the highlighted row — so any row counts, not just
		// the lit one. Tab from the search field lands on the first row while the
		// highlight rests on the best match, and testing only the lit row would leave
		// focus behind and activate the wrong one.
		const focusedId = document.activeElement?.id
		const rowWasFocused =
			!!focusedId && Array.from({ length: count }, (_, i) => opts.rowId(i)).includes(focusedId)
		index = index < 0 ? (delta > 0 ? 0 : count - 1) : (index + delta + count) % count
		const row = document.getElementById(opts.rowId(index))
		row?.scrollIntoView({ block: 'nearest' })
		if (rowWasFocused) row?.focus()
	}

	return {
		get index() {
			return index
		},
		/** Wire to each row's `onMouseEnter`. */
		hovered(i: number) {
			if (pointerOwns) index = i
		},
		/** Wire to the list container's `onpointermove`. */
		pointerMoved() {
			pointerOwns = true
		},
		/** Wire to the container that holds the search field and the rows, so the keys are
		 * answered whichever of the two has focus. */
		onKeydown(e: KeyboardEvent) {
			if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
				e.preventDefault()
				move(e.key === 'ArrowDown' ? 1 : -1)
			} else if (
				e.key === 'Enter' &&
				opts.activateEnterFrom?.includes((e.target as HTMLElement)?.id) &&
				index >= 0
			) {
				e.preventDefault()
				opts.onActivate(index)
			}
		}
	}
}
