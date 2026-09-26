import { tick } from 'svelte'

/**
 * Resolve on the next animation frame, or immediately where there is no rAF (SSR).
 */
export function nextAnimationFrame(): Promise<void> {
	return new Promise((resolve) => {
		if (typeof requestAnimationFrame === 'function') {
			requestAnimationFrame(() => resolve())
		} else {
			resolve()
		}
	})
}

/**
 * Flush pending DOM updates, then wait until the browser has painted them.
 *
 * A lazily-mounted drawer creates its panel and gets its `open` class in the same
 * flush. A CSS transition takes its start value from the last painted computed
 * style, so without a painted closed state the panel snaps open instead of sliding.
 * `tick()` alone only flushes the DOM, it does not wait for a paint.
 *
 * The second frame is load-bearing: a requestAnimationFrame callback runs *before*
 * the paint it is scheduled against, so one frame still leaves the closed state
 * unpainted. Measured — with a single frame the panel does not transition at all.
 */
export async function tickPainted(): Promise<void> {
	await tick()
	await nextAnimationFrame()
	await nextAnimationFrame()
}

const QUIET_MS = 300
const MAX_WAIT_MS = 5000
let pageIdle: Promise<void> | undefined

/**
 * Resolve once the first page has loaded and its network has gone quiet (no resource
 * finished for `QUIET_MS`, capped at `MAX_WAIT_MS` after `load`), then the main thread
 * is idle. Gate prefetches of chunks the page does not need yet on it: started any
 * earlier they compete with the page's own chunks and API calls for connections and
 * bandwidth, which delays its first content. Resolves once per tab. A `load` that never
 * fires (a hanging image or iframe) starts the wait anyway after `MAX_WAIT_MS`.
 */
export function whenPageIdle(): Promise<void> {
	return (pageIdle ??= new Promise((resolve) => {
		if (typeof window === 'undefined') return resolve()
		let settled = false
		const settle = () => {
			if (settled) return
			settled = true
			const loadedAt = performance.now()
			let timer: ReturnType<typeof setTimeout> | undefined
			let observer: PerformanceObserver | undefined
			let finished = false
			const done = () => {
				if (finished) return
				finished = true
				observer?.disconnect()
				clearTimeout(timer)
				if (typeof requestIdleCallback === 'function') {
					requestIdleCallback(() => resolve(), { timeout: 1000 })
				} else {
					resolve()
				}
			}
			const arm = () => {
				if (finished) return
				clearTimeout(timer)
				const left = MAX_WAIT_MS - (performance.now() - loadedAt)
				timer = setTimeout(done, Math.max(0, Math.min(QUIET_MS, left)))
			}
			try {
				observer = new PerformanceObserver(arm)
				observer.observe({ type: 'resource' })
			} catch {}
			arm()
		}
		if (document.readyState === 'complete') settle()
		else {
			window.addEventListener('load', settle, { once: true })
			setTimeout(settle, MAX_WAIT_MS)
		}
	}))
}
