// The layers the composer's controls open render in portals under <body>, outside the
// composer's own element: DropdownV2 menus, melt popovers, and the settings modals. Not
// `[data-menu]`: DropdownV2 puts it on its trigger too, and Home has triggers everywhere.
// `.wm-modal-form-popup` is Modal2's box, which carries no dialog role: without it a press
// inside Assistant settings folds the composer, and the modal goes with it.
const OVERLAY =
	'[data-melt-dropdown-menu], [data-melt-popover-content], [role="dialog"], .wm-modal-form-popup'

/** Whether a pointer-down outside the composer still leaves it open: it landed in a menu or
 * dialog, or one is open, and the press is closing it rather than leaving the composer. */
export function isComposerOverlay(target: EventTarget | null): boolean {
	if (target instanceof Element && target.closest(OVERLAY)) return true
	return document.querySelector(OVERLAY) !== null
}

/** Whether a pointer-down outside the composer leaves it open: it is on a control that brings
 * the user to it (`data-asks-for-composer`, New session), where folding it on the way in would
 * collapse it and open it again under the user's eyes, or on what the user works in alongside
 * it (`data-works-with-composer`, Home's lists), where a click adds an item to it. */
export function asksForComposer(target: EventTarget | null): boolean {
	return (
		target instanceof Element &&
		target.closest('[data-asks-for-composer], [data-works-with-composer]') !== null
	)
}
