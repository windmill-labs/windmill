import { isMac } from '$lib/utils'

const state = $state({ held: false })

if (typeof window !== 'undefined') {
	// Only the modifier that actually yields a tab: shift opens a window, alt can start a
	// download, and on macOS ctrl+click is a secondary click. An affordance keyed on any of
	// those would promise a destination the browser never opens.
	//
	// Read the flag off each event instead of accumulating keydown/keyup pairs. Every key event
	// carries the full modifier state, so an event lost to a focus change cannot strand the flag
	// on; an accumulated set has no way back once it misses a keyup.
	const sync = (event: KeyboardEvent) => {
		state.held = isMac() ? event.metaKey : event.ctrlKey
	}
	const clear = () => {
		state.held = false
	}
	window.addEventListener('keydown', sync)
	window.addEventListener('keyup', sync)
	// A modifier released after focus left delivers its keyup elsewhere, so without these the
	// last event this window saw would hold the flag on until the user pressed a key again.
	window.addEventListener('blur', clear)
	document.addEventListener('visibilitychange', () => {
		if (document.hidden) clear()
	})
}

/**
 * True while the modifier that turns a click into a new browser tab is held, as last seen by
 * this window. A modifier pressed while the window was unfocused reads as false until the next
 * key or focus event: the browser delivers no key events to a window that is not focused.
 */
export const newTabModifier = state
