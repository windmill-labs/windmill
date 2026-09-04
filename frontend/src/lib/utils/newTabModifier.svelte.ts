import { isMac } from '$lib/utils'

const state = $state({ held: false })

if (typeof window !== 'undefined') {
	// Only the modifier that actually yields a tab: shift opens a window, alt can start a
	// download, and on macOS ctrl+click is a secondary click.
	// Taken from each event rather than accumulated across keydown/keyup pairs, so a keyup lost
	// to a focus change cannot strand the flag on.
	const sync = (event: KeyboardEvent) => {
		state.held = isMac() ? event.metaKey : event.ctrlKey
	}
	const clear = () => {
		state.held = false
	}
	// Capture: editors and menus stopPropagation the keys they handle, hiding the modifier from
	// a bubble-phase listener whenever focus sits in one.
	window.addEventListener('keydown', sync, true)
	window.addEventListener('keyup', sync, true)
	// Not capture, unlike the two above: blur does not bubble but does reach the window while
	// capturing, so it would fire for every element that loses focus. Needed at all because a
	// modifier released after focus left delivers its keyup elsewhere.
	window.addEventListener('blur', clear)
	document.addEventListener('visibilitychange', () => {
		if (document.hidden) clear()
	})
}

/**
 * True while the modifier that turns a click into a new browser tab is held, as last seen by
 * this window. A modifier pressed while the window was unfocused reads as false until the next
 * key event: the browser delivers no key events to a window that is not focused, and nothing
 * here re-reads the modifier on focus.
 */
export const newTabModifier = state
