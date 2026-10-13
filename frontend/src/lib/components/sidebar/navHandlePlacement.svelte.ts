import { BAR_HEIGHT_PX } from '../pageHeaderRegistry.svelte'

// Opening the detached sidebar's card, from the band's own handle or from the corner control that
// stands in for it on a page which hides the band.
let opener: (() => void) | undefined
// Closing it once the pointer is off both the handle and the card, which the layout owns because
// the card is its markup.
let hoverClose: { schedule: () => void; cancel: () => void } | undefined
let card: { isOpen: () => boolean; close: () => void } | undefined
let peeked = false

// Where the card's top edge goes: the header's bottom edge, so the two meet in a line rather than
// the card starting at whatever height the control that opened it happens to have. Measured at open
// time rather than fixed, so the card follows the header's own height. On a page that hides the
// header, the header is still what the card will sit under — it comes down with it — so its height
// is read even while it is off-screen. The fallback matches the header's resting height.
const FALLBACK_CARD_TOP = BAR_HEIGHT_PX
const GAP_UNDER_HANDLE = 6
let cardTop = $state(FALLBACK_CARD_TOP)

export const navHandleSlot = {
	/** Distance from the top of the viewport to the card's top edge, in px. */
	get cardTop(): number {
		return cardTop
	},
	/** The root layout owns the card, so it registers how to open, close and hover-close it. */
	setOpener(
		open: () => void,
		close?: { schedule: () => void; cancel: () => void },
		state?: { isOpen: () => boolean; close: () => void }
	) {
		opener = open
		hoverClose = close
		card = state
	},
	/** The pointer entered the handle. */
	peek() {
		peeked = !card?.isOpen()
		this.open()
	},
	/**
	 * A click on the handle: closes an open card, opens a closed one. The first click after the
	 * pointer opened the card leaves it open — that click was aimed at a closed card, and a touch
	 * tap sends the mouseenter that opens it just before its click.
	 */
	toggle(anchor?: Element | null) {
		const wasPeek = peeked
		peeked = false
		if (wasPeek && card?.isOpen()) return
		if (card?.isOpen()) {
			hoverClose?.cancel()
			card.close()
		} else {
			this.open(anchor)
		}
	},
	/** Pointer left the handle: close, unless it lands on the card before the delay is up. */
	scheduleClose() {
		hoverClose?.schedule()
	},
	/**
	 * Opens the card under the page header, or under `anchor` when there is no header to sit
	 * beneath — an embed, where whatever reached for the nav is all the card has to hang from.
	 */
	open(anchor?: Element | null) {
		hoverClose?.cancel()
		const band = document.querySelector('[data-page-header]')
		const bandHeight = band?.getBoundingClientRect().height
		if (bandHeight) {
			cardTop = Math.round(bandHeight)
		} else {
			const bottom = anchor?.getBoundingClientRect().bottom
			cardTop =
				bottom != null && bottom > 0 ? Math.round(bottom) + GAP_UNDER_HANDLE : FALLBACK_CARD_TOP
		}
		opener?.()
	}
}
