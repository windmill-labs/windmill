/**
 * Keeps a growing pane pinned to its end, for the chat transcript and the agent run
 * viewer. Shared for one non-obvious guard: a programmatic scroll dispatches its
 * `scroll` event asynchronously, so content landing in between widens the gap for a
 * tick, which reads as the reader scrolling away and disengages the follow.
 */

/**
 * Distance from the end within which a reader counts as still following. Allows
 * for sub-pixel rounding from `scrollTo` and the occasional overscroll bounce.
 */
const STICK_TO_BOTTOM_PX = 8

/** A scroll event this close after our own scroll is ours, not the reader's. */
const OWN_SCROLL_WINDOW_MS = 120

export type BottomSticker = {
	/** Jump to the end. Instant: smooth would animate every append and race the next. */
	scrollToEnd: (pane: HTMLElement | undefined | null) => void
	/** Whether the pane is at its end, i.e. the reader wants to be carried along. */
	isAtEnd: (pane: HTMLElement | undefined | null) => boolean
	/** Whether the scroll event being handled was one we caused. */
	isOwnScroll: () => boolean
}

export function createBottomSticker(): BottomSticker {
	let scrolledAt: number | undefined

	return {
		scrollToEnd(pane) {
			if (!pane) {
				return
			}
			scrolledAt = Date.now()
			pane.scrollTo({ top: pane.scrollHeight, behavior: 'auto' })
		},
		isAtEnd(pane) {
			if (!pane) {
				return false
			}
			return pane.scrollHeight - pane.scrollTop - pane.clientHeight <= STICK_TO_BOTTOM_PX
		},
		isOwnScroll() {
			return scrolledAt !== undefined && Date.now() - scrolledAt < OWN_SCROLL_WINDOW_MS
		}
	}
}
