/**
 * The pane an agent run is rendered in. Selected on `overflow-y` alone, never on
 * whether the element currently overflows: a pane that fits its content is still
 * the pane, and an overflow test walks past it into `#content`, which overflows
 * merely because the document scrolls — scrolling that drops the whole page.
 */
export function runPane(node: HTMLElement | undefined | null): HTMLElement | undefined {
	let current = node?.parentElement
	while (current && current !== document.body && current.id !== 'content') {
		const overflowY = getComputedStyle(current).overflowY
		if (overflowY === 'auto' || overflowY === 'scroll') {
			return current
		}
		current = current.parentElement
	}
	return undefined
}
