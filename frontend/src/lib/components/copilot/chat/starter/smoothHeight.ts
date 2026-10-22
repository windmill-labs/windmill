type SmoothHeightParams = {
	/** Off: the element takes its natural height and nothing is animated. */
	enabled: boolean
	/** The state whose change should animate. Height changes at any other time follow the
	 * content at once: animating every line the user types would clip the caret's line. */
	key: unknown
}

const DURATION_MS = 200
const EASING = 'cubic-bezier(0.2, 0.8, 0.2, 1)'

/**
 * Sizes `node` to its first child and animates that height when `key` changes, so a block
 * that changes shape grows and shrinks instead of jumping. The child is measured, not the
 * node: the node's own height is the one being set. `node` clips while it animates, so a
 * focus ring belongs on `node` itself (box-shadow is not clipped by its own overflow).
 */
export function smoothHeight(node: HTMLElement, params: SmoothHeightParams) {
	let { enabled, key } = params
	let animateUntil = 0
	const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)')

	const apply = () => {
		const inner = node.firstElementChild as HTMLElement | null
		if (!enabled || !inner) {
			node.style.height = ''
			node.style.overflow = ''
			node.style.transition = ''
			return
		}
		const borders = node.offsetHeight - node.clientHeight
		const animate = performance.now() < animateUntil && !reducedMotion.matches
		node.style.overflow = 'hidden'
		node.style.transition = animate ? `height ${DURATION_MS}ms ${EASING}` : 'none'
		node.style.height = `${inner.offsetHeight + borders}px`
	}

	// Created on first use: most chats mount this disabled and never turn it on.
	let ro: ResizeObserver | undefined
	const observe = () => {
		ro?.disconnect()
		const inner = node.firstElementChild
		if (!enabled || !inner) return
		ro ??= new ResizeObserver(apply)
		ro.observe(inner)
	}
	// No measuring at mount: the content's height is not settled yet (a newly mounted session
	// composer reads ~60px too tall), and a view transition captures the box at that height.
	// The observer's first callback comes after layout and sets the right one.
	observe()

	return {
		update(next: SmoothHeightParams) {
			if (next.key !== key) animateUntil = performance.now() + DURATION_MS
			;({ enabled, key } = next)
			observe()
			apply()
		},
		destroy() {
			ro?.disconnect()
		}
	}
}
