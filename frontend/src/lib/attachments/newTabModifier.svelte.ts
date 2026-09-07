import type { Attachment } from 'svelte/attachments'
import { isMac } from '$lib/utils'

/**
 * Tracks whether the modifier that turns a click into a new browser tab is held, but only while
 * the attached element is hovered, which is the only moment the answer is used.
 */
export function newTabModifier() {
	let held = $state(false)

	// Only the modifier that actually yields a tab: shift opens a window, alt can start a
	// download, and on macOS ctrl+click is a secondary click.
	// Taken from each event rather than accumulated across keydown/keyup pairs, so a keyup lost to
	// a focus change cannot strand the flag on.
	const sync = (event: KeyboardEvent | MouseEvent) => {
		held = isMac() ? event.metaKey : event.ctrlKey
	}
	const clear = () => {
		held = false
	}

	const attach: Attachment<HTMLElement> = (node) => {
		// One controller per hover: a mirrored remove list leaks any listener whose options drift.
		let hover: AbortController | undefined
		const leave = () => {
			hover?.abort()
			hover = undefined
			clear()
		}
		const enter = (event: MouseEvent) => {
			// Seeded from the hover itself: mouse events carry the same modifier flags as key events,
			// so a modifier already held before the pointer arrived reads correctly.
			sync(event)
			hover = new AbortController()
			const { signal } = hover
			// Same reason the hover seeds: a modifier held across a keyboard app switch delivers no
			// keydown on the way back, so the pointer is all that is left to re-read it from.
			node.addEventListener('mousemove', sync, { signal })
			// Capture: editors and menus stopPropagation the keys they handle, hiding the modifier
			// from a bubble-phase listener whenever focus sits in one.
			window.addEventListener('keydown', sync, { capture: true, signal })
			window.addEventListener('keyup', sync, { capture: true, signal })
			// Not capture, unlike the two above: blur does not bubble but does reach the window while
			// capturing, so it would fire for every element that loses focus.
			window.addEventListener('blur', clear, { signal })
		}

		const life = new AbortController()
		node.addEventListener('mouseenter', enter, { signal: life.signal })
		node.addEventListener('mouseleave', leave, { signal: life.signal })
		return () => {
			life.abort()
			leave()
		}
	}

	return {
		get held() {
			return held
		},
		attach
	}
}
