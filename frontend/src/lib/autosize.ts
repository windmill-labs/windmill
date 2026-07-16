import { tick } from 'svelte'

type TextArea = HTMLTextAreaElement

/**
 * Optional parameters for the `autosize` action.
 *
 * `maxHeight` caps how tall the textarea may grow. Once the content exceeds it,
 * the textarea stops growing and scrolls internally (overflow-y: auto) instead.
 * Accepts a number (px) or a CSS-ish string ending in `vh`/`px` (e.g. `'40vh'`).
 * When omitted the textarea grows without bound (the historical behaviour).
 *
 * `minHeight` overrides the floor the textarea shrinks to (default 30px). A
 * compact single-line input wants a smaller floor so the line isn't padded with
 * dead space below it.
 */
export type AutosizeParams = { maxHeight?: number | string; minHeight?: number } | undefined

/** Resolve a `maxHeight` param to a pixel value, or null when uncapped/invalid. */
function resolveMaxHeight(maxHeight: number | string | undefined): number | null {
	if (maxHeight == null) return null
	if (typeof maxHeight === 'number') return maxHeight
	const s = maxHeight.trim()
	const v = parseFloat(s)
	if (isNaN(v)) return null
	if (s.endsWith('vh')) return (v / 100) * window.innerHeight
	return v // 'px' or bare number → pixels
}

export const autosize = (node: TextArea, params?: AutosizeParams) => {
	/* ------------------------------------------------------------------
	 * Constants
	 * ---------------------------------------------------------------- */
	const UPDATE_EVENT = new Event('update')
	const DEFAULT_MIN_HEIGHT = 30 // px
	const EXTRA = 2 // px added to scrollHeight

	let width = 0
	let maxHeight = params?.maxHeight
	let minHeight = params?.minHeight ?? DEFAULT_MIN_HEIGHT
	let capped = maxHeight != null

	/* ------------------------------------------------------------------
	 * Core resize routine
	 * ---------------------------------------------------------------- */
	const resize = () => {
		node.style.height = 'auto'
		let height = Math.max(node.scrollHeight, minHeight) + EXTRA

		const maxPx = resolveMaxHeight(maxHeight)
		if (maxPx != null) {
			if (height > maxPx) {
				height = maxPx
				node.style.overflowY = 'auto'
			} else {
				node.style.overflowY = 'hidden'
			}
		} else {
			// Uncapped — including after a capped→uncapped toggle: drop any inline
			// overflow we set while capped so the textarea returns to its default
			// (class-driven) behaviour rather than keeping a stale `auto`/`hidden`.
			node.style.overflowY = ''
		}

		node.style.height = `${height}px`
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

	// A `vh`-based cap depends on the viewport height, so recompute on window
	// resize. Only attached when a cap is configured to avoid adding listeners
	// for the many uncapped textareas across the app.
	if (capped) {
		window.addEventListener('resize', resize)
	}

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
	const ro = new ResizeObserver(([entry]) => {
		const newWidth = entry.contentRect.width
		if (newWidth !== width) {
			width = newWidth
			resize()
		}
	})
	ro.observe(node)

	/* ------------------------------------------------------------------
	 * Action lifecycle
	 * ---------------------------------------------------------------- */
	return {
		update(newParams?: AutosizeParams) {
			maxHeight = newParams?.maxHeight
			minHeight = newParams?.minHeight ?? DEFAULT_MIN_HEIGHT
			const nowCapped = maxHeight != null
			if (nowCapped && !capped) {
				window.addEventListener('resize', resize)
			} else if (!nowCapped && capped) {
				window.removeEventListener('resize', resize)
			}
			capped = nowCapped
			resize()
		},
		destroy() {
			ro.disconnect()
			node.removeEventListener('input', onInput)
			node.removeEventListener('update', resize)
			if (capped) {
				window.removeEventListener('resize', resize)
			}
		}
	}
}

export default autosize
