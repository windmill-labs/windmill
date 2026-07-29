import { cubicOut } from 'svelte/easing'
import type { EasingFunction, TransitionConfig } from 'svelte/transition'

/** `slide`, but re-measuring the content on every frame.
 *
 *  `slide` snapshots the height once when the transition starts, so content that settles
 *  after mount (a Monaco editor sizing itself to its lines) animates towards a stale
 *  target and snaps to its real height at the end. */
export function slideDynamic(
	node: HTMLElement,
	{
		delay = 0,
		duration = 150,
		easing = cubicOut
	}: { delay?: number; duration?: number; easing?: EasingFunction } = {}
): TransitionConfig {
	const style = getComputedStyle(node)
	const paddingTop = parseFloat(style.paddingTop)
	const paddingBottom = parseFloat(style.paddingBottom)
	const initial = {
		overflow: node.style.overflow,
		height: node.style.height,
		paddingTop: node.style.paddingTop,
		paddingBottom: node.style.paddingBottom
	}
	return {
		delay,
		duration,
		easing,
		tick: (t: number) => {
			if (t === 1) {
				Object.assign(node.style, initial)
				return
			}
			node.style.overflow = 'hidden'
			// Padding shrinks with the box, or `border-box` would floor the height at it.
			node.style.paddingTop = `${t * paddingTop}px`
			node.style.paddingBottom = `${t * paddingBottom}px`
			// scrollHeight ignores the height clamp but does count the padding just written.
			const content = node.scrollHeight - t * (paddingTop + paddingBottom)
			node.style.height = `${t * (content + paddingTop + paddingBottom)}px`
		}
	}
}
