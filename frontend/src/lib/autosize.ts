import { tick } from 'svelte'

type TextArea = HTMLTextAreaElement

export const autosize = (node: TextArea) => {
	/* ------------------------------------------------------------------
	 * Constants
	 * ---------------------------------------------------------------- */
	const UPDATE_EVENT = new Event('update')
	const MIN_HEIGHT = 30 // px
	const EXTRA = 2 // px added to scrollHeight

	/* ------------------------------------------------------------------
	 * Core resize routine
	 * ---------------------------------------------------------------- */
	const resize = () => {
		node.style.height = 'auto'
		node.style.height = `${Math.max(node.scrollHeight, MIN_HEIGHT) + EXTRA}px`
	}

	/* ------------------------------------------------------------------
	 * Patch `value` so programmatic changes trigger resize
	 * ---------------------------------------------------------------- */
	const proto = Object.getPrototypeOf(node)
	const desc = Object.getOwnPropertyDescriptor(proto, 'value')

	if (desc) {
		Object.defineProperty(node, 'value', {
			get() {
				return desc.get?.call(this)
			},
			set(v: unknown) {
				desc.set?.call(this, v)
				node.dispatchEvent(UPDATE_EVENT)
			}
		})
	}

	/* ------------------------------------------------------------------
	 * Event listeners
	 * ---------------------------------------------------------------- */
	const onInput = () => node.dispatchEvent(UPDATE_EVENT)

	node.addEventListener('input', onInput)
	node.addEventListener('update', resize)

	/* ------------------------------------------------------------------
	 * Inline styling
	 * ---------------------------------------------------------------- */
	node.style.boxSizing = 'border-box'

	/* ------------------------------------------------------------------
	 * Wait for DOM mount, then do an initial measure.
	 * If the <textarea> is already visible (offsetWidth > 0) this covers it;
	 * otherwise the first ResizeObserver callback will.
	 * ---------------------------------------------------------------- */
	;(async () => {
		await tick()
		resize()
	})()

	/* ------------------------------------------------------------------
	 * ResizeObserver – handles:
	 *   • first time the element gets a real width
	 *   • container/window resizes afterwards
	 * ---------------------------------------------------------------- */
	const ro = new ResizeObserver(resize)
	ro.observe(node)

	/* ------------------------------------------------------------------
	 * Action lifecycle
	 * ---------------------------------------------------------------- */
	return {
		destroy() {
			ro.disconnect()
			node.removeEventListener('input', onInput)
			node.removeEventListener('update', resize)
		}
	}
}

export default autosize
