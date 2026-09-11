/**
 * Scroll the pane an agent run is rendered in to its end.
 *
 * A run reads in the order it happened, so its output is the last thing in the
 * list — landing at the bottom is landing on the answer, and while it streams
 * that is also where the text is arriving.
 *
 * Deliberately the nearest *scrollable* ancestor rather than `scrollIntoView`:
 * the result viewer sits inside pages that scroll themselves, and yanking a
 * whole run page to the middle because a step returned an agent result would be
 * worse than not scrolling at all.
 */
export function scrollPaneToEnd(anchor: HTMLElement | undefined | null) {
	let node = anchor?.parentElement
	while (node && node !== document.body) {
		const overflowY = getComputedStyle(node).overflowY
		if ((overflowY === 'auto' || overflowY === 'scroll') && node.scrollHeight > node.clientHeight) {
			node.scrollTop = node.scrollHeight
			return
		}
		node = node.parentElement
	}
}
