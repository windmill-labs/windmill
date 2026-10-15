import { untrack } from 'svelte'
import { useReducedMotion } from '$lib/svelte5Utils.svelte'

/** The glide's timing, read from the tokens app.css defines once for this move and for
 * the Home ↔ AI Sessions view transition, so the two read as the same motion. */
function glideTiming(): { duration: number; easing: string } {
	const style = getComputedStyle(document.documentElement)
	const duration = parseFloat(style.getPropertyValue('--composer-glide-duration'))
	const easing = style.getPropertyValue('--composer-glide-easing').trim()
	return { duration: Number.isFinite(duration) ? duration : 200, easing: easing || 'ease-out' }
}

/**
 * Slides `el` from where it sat to where it lands whenever `layout()` changes (an empty
 * session's composer dropping to the bottom on its first message). Measured before the DOM
 * updates, again after, and the difference animated away; an element that did not move does
 * not animate. Under reduced motion it fades in where it landed.
 */
export function glideOnLayoutChange(el: () => HTMLElement | undefined, layout: () => unknown) {
	const reducedMotion = useReducedMotion()
	let from: DOMRect | undefined
	$effect.pre(() => {
		layout()
		from = untrack(el)?.getBoundingClientRect()
	})
	$effect(() => {
		layout()
		const node = untrack(el)
		const start = from
		from = undefined
		if (!node || !start) return
		const { duration, easing } = glideTiming()
		if (reducedMotion.val) {
			node.animate([{ opacity: 0 }, { opacity: 1 }], { duration })
			return
		}
		const end = node.getBoundingClientRect()
		const dx = start.left - end.left
		const dy = start.top - end.top
		if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return
		node.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: 'none' }], {
			duration,
			easing
		})
	})
}
