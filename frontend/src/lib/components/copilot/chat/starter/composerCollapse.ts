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
