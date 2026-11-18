import { BAR_HEIGHT_PX } from '../pageHeaderRegistry.svelte'
import type { Item } from '$lib/utils'

// Opening the detached sidebar's card, from the band's own handle or from the corner control that
// stands in for it on a page which hides the band.
let opener: (() => void) | undefined
// Closing it once the pointer is off both the handle and the card, which the layout owns because
// the card is its markup.
let hoverClose: { schedule: () => void; cancel: () => void } | undefined
type CardState = {
	isOpen: () => boolean
	isPinned: () => boolean
	pin: () => void
	close: () => void
}
let card: CardState | undefined

// Where the card's top edge goes: the header's bottom edge, so the two meet in a line rather than
// the card starting at whatever height the control that opened it happens to have. Measured at open
// time rather than fixed, so the card follows the header's own height. On a page that hides the
// header, the header is still what the card will sit under — it comes down with it — so its height
// is read even while it is off-screen. The fallback matches the header's resting height.
const FALLBACK_CARD_TOP = BAR_HEIGHT_PX
const GAP_UNDER_HANDLE = 6
let cardTop = $state(FALLBACK_CARD_TOP)
// The sidebar mode menu, which the layout builds from its own state. Where the card slides in on
// hover, the handle's click opens it, as the card then carries no footer to hold it.
let modeItems: (() => Item[]) | undefined = $state()
// A mode change can move the mode button between the sidebar's footer and the page header, so the
// button flashes where it lands. Nonzero only while the flash plays: the header remounts on every
// page, and a lasting value would replay it there.
const MODE_FLASH_MS = 1500
let modeFlash = $state(0)
let modeFlashTimer: ReturnType<typeof setTimeout> | undefined

export const navHandleSlot = {
	/** Distance from the top of the viewport to the card's top edge, in px. */
	get cardTop(): number {
		return cardTop
	},
	/** The card is open and pinned, so the handle shows as selected. Reactive. */
	get pinned(): boolean {
		return !!card?.isOpen() && card.isPinned()
	},
	/** The sidebar mode menu's items, once the layout has registered them. Reactive. */
	get modeItems(): Item[] | undefined {
		return modeItems?.()
	},
	setModeItems(items: () => Item[]) {
		modeItems = items
	},
	/** Nonzero while the mode button flashes after a mode change; each change gets a new value. */
	get modeFlash(): number {
		return modeFlash
	},
	flashModeButton() {
		clearTimeout(modeFlashTimer)
		modeFlash += 1
		modeFlashTimer = setTimeout(() => (modeFlash = 0), MODE_FLASH_MS)
	},
	/** The root layout owns the card, so it registers how to open, pin, close and hover-close it. */
	setOpener(
		open: () => void,
		close?: { schedule: () => void; cancel: () => void },
		state?: CardState
	) {
		opener = open
		hoverClose = close
		card = state
	},
	/**
	 * A click on the handle pins the card, opening it first if it is closed: a pinned card stays
	 * when the pointer leaves, until a press on the page, Esc, or the next click here closes it. A
	 * card the pointer opened is not pinned yet, so the click that follows the hover pins it — and
	 * a touch tap, which sends that mouseenter just before its click, pins it too.
	 */
	toggle(anchor?: Element | null) {
		if (card?.isOpen() && card.isPinned()) {
			card.close()
			return
		}
		if (!card?.isOpen()) this.open(anchor)
		hoverClose?.cancel()
		card?.pin()
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
