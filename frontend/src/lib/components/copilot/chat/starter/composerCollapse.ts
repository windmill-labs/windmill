/** Whether Home's composer rests as one line. Anything the user has started (text, an
 * attachment) or is in the middle of (they opened it and have not left) keeps it open, and so
 * does a workspace small enough that the composer is the page's main way in. */
export function composerCollapses(s: {
	onHome: boolean
	usable: boolean
	fewItems: boolean
	opened: boolean
	draftEmpty: boolean
}): boolean {
	return s.onHome && s.usable && !s.fewItems && !s.opened && s.draftEmpty
}

// The layers the composer's controls open render in portals under <body>, outside the
// composer's own element: DropdownV2 menus, melt popovers, and the settings modals. Not
// `[data-menu]`: DropdownV2 puts it on its trigger too, and Home has triggers everywhere.
const OVERLAY = '[data-melt-dropdown-menu], [data-melt-popover-content], [role="dialog"]'

/** Whether a pointer-down outside the composer still leaves it open: it landed in a menu or
 * dialog, or one is open, and the press is closing it rather than leaving the composer. */
export function isComposerOverlay(target: EventTarget | null): boolean {
	if (target instanceof Element && target.closest(OVERLAY)) return true
	return document.querySelector(OVERLAY) !== null
}
