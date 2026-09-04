import type { Attachment } from 'svelte/attachments'
import { isMac } from '$lib/utils'

const state = $state({ held: false })

// Only the modifier that actually yields a tab: shift opens a window, alt can start a
// download, and on macOS ctrl+click is a secondary click.
// Taken from each event rather than accumulated across keydown/keyup pairs, so a keyup lost to
// a focus change cannot strand the flag on.
const sync = (event: KeyboardEvent | MouseEvent) => {
	state.held = isMac() ? event.metaKey : event.ctrlKey
}
const clear = () => {
	state.held = false
}

// One hovered element at a time owns the listeners. Guarded by node identity because a pill can
// be destroyed while a different one is hovered, and its teardown must not stop that one.
let hovered: HTMLElement | undefined

function stop(node: HTMLElement) {
	if (hovered !== node) return
	hovered = undefined
	window.removeEventListener('keydown', sync, true)
	window.removeEventListener('keyup', sync, true)
	window.removeEventListener('blur', clear)
	clear()
}

/**
 * Tracks whether the modifier that turns a click into a new browser tab is held, but only while
 * the attached element is hovered, which is the only moment the answer is used.
 *
 *   <span {@attach trackNewTabModifier}>…</span>
 */
export const trackNewTabModifier: Attachment<HTMLElement> = (node) => {
	const enter = (event: MouseEvent) => {
		hovered = node
		// Seeded from the hover itself: mouse events carry the same modifier flags as key events,
		// so a modifier already held before the pointer arrived reads correctly.
		sync(event)
		// Capture: editors and menus stopPropagation the keys they handle, hiding the modifier
		// from a bubble-phase listener whenever focus sits in one.
		window.addEventListener('keydown', sync, true)
		window.addEventListener('keyup', sync, true)
		// Not capture, unlike the two above: blur does not bubble but does reach the window while
		// capturing, so it would fire for every element that loses focus.
		window.addEventListener('blur', clear)
	}
	const leave = () => stop(node)

	node.addEventListener('mouseenter', enter)
	node.addEventListener('mouseleave', leave)
	return () => {
		node.removeEventListener('mouseenter', enter)
		node.removeEventListener('mouseleave', leave)
		stop(node)
	}
}

/** True while the new-tab modifier is held over an element tracked by {@link trackNewTabModifier}. */
export const newTabModifier = state
