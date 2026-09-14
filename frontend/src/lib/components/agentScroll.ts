/**
 * The pane an agent run is rendered in: the nearest ancestor that is a real
 * scroll container.
 *
 * Selection is on `overflow-y` alone, deliberately *not* on whether the element
 * currently overflows. A pane that happens to fit its content is still the pane,
 * and skipping it walks straight past into page chrome — `#content` and `<body>`
 * report `scrollHeight > clientHeight` merely because the document scrolls, so an
 * overflow test picks them and scrolling one drops a whole run page to its
 * footer. Their `overflow-y` is `visible`, which is what actually distinguishes
 * them, and the walk stops at the page's content root either way.
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
