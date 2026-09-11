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

/** Scroll the run's pane to its end, where its output is. */
export function scrollRunToEnd(node: HTMLElement | undefined | null) {
	const pane = runPane(node)
	if (pane) {
		pane.scrollTop = pane.scrollHeight
	}
}

/** Within this many pixels of the end, a reader is still following along. */
const FOLLOWING_PX = 32

/**
 * Whether the reader is still at the end and so wants new content to carry them
 * with it. Someone who has scrolled up to read an earlier row is reading it, and
 * must not be dragged back on the next poll.
 */
export function isFollowingEnd(node: HTMLElement | undefined | null): boolean {
	const pane = runPane(node)
	if (!pane) {
		return false
	}
	return pane.scrollHeight - pane.scrollTop - pane.clientHeight <= FOLLOWING_PX
}
