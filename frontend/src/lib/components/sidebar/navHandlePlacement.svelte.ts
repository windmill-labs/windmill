// Opening the detached sidebar's card, from the handle or from anywhere else that reaches for the
// nav (the left edge band, the AI chat layout's menu button).
let opener: (() => void) | undefined

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
	/** The root layout owns the card, so it registers how to open it. */
	setOpener(open: () => void) {
		opener = open
	},
	open() {
		const handle = document.querySelector('[data-nav-handle]')
		cardTop = handle
			? Math.round(handle.getBoundingClientRect().bottom) + GAP_UNDER_HANDLE
			: FALLBACK_CARD_TOP
		opener?.()
	}
}
