// Opening the detached sidebar's card, from the handle or from anywhere else that reaches for the
// nav (the left edge band, the AI chat layout's menu button).
let opener: (() => void) | undefined
// Closing it once the pointer is off both the handle and the card, which the layout owns because
// the card is its markup.
let hoverClose: { schedule: () => void; cancel: () => void } | undefined

// Where the card's top edge goes: right under the handle, measured when it opens rather than fixed,
// so the card follows the handle's own height. The fallback matches the handle's resting bottom.
const FALLBACK_CARD_TOP = 44
const GAP_UNDER_HANDLE = 6
let cardTop = $state(FALLBACK_CARD_TOP)

export const navHandleSlot = {
	/** Distance from the top of the viewport to the card's top edge, in px. */
	get cardTop(): number {
		return cardTop
	},
	/** The root layout owns the card, so it registers how to open and hover-close it. */
	setOpener(open: () => void, close?: { schedule: () => void; cancel: () => void }) {
		opener = open
		hoverClose = close
	},
	/** Pointer left the handle: close, unless it lands on the card before the delay is up. */
	scheduleClose() {
		hoverClose?.schedule()
	},
	/**
	 * Opens the card under `anchor`, or under the handle in the page header when none is given.
	 * A page that hides the header reveals the sidebar from its own control, and the card has to
	 * hang from the control the pointer is actually on — the header's handle is off-screen there.
	 */
	open(anchor?: Element | null) {
		hoverClose?.cancel()
		const handle = anchor ?? document.querySelector('[data-nav-handle]')
		const bottom = handle?.getBoundingClientRect().bottom
		cardTop =
			bottom != null && bottom > 0 ? Math.round(bottom) + GAP_UNDER_HANDLE : FALLBACK_CARD_TOP
		opener?.()
	}
}
