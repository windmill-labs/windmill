/** `querySelector` that also descends into open shadow roots, where Pierre renders its lines. */
function findDeep(root: ParentNode, selector: string): Element | null {
	const direct = root.querySelector(selector)
	if (direct) return direct
	for (const el of Array.from(root.querySelectorAll('*'))) {
		if (el.shadowRoot) {
			const found = findDeep(el.shadowRoot, selector)
			if (found) return found
		}
	}
	return null
}

/** Resolves with `performance.now()` at the first frame where `selector` matches under `root`. */
export function waitForSelector(
	root: () => HTMLElement | undefined,
	selector: string,
	timeoutMs = 30_000
): Promise<number | undefined> {
	const start = performance.now()
	return new Promise((resolve) => {
		const check = () => {
			const r = root()
			if (r && findDeep(r, selector)) return resolve(performance.now())
			if (performance.now() - start > timeoutMs) return resolve(undefined)
			requestAnimationFrame(check)
		}
		check()
	})
}

/**
 * Sums main-thread long tasks (>50ms) until `stop()`. Chromium only; elsewhere the total
 * stays undefined.
 */
export function observeLongTasks(): { stop: () => number | undefined } {
	if (!PerformanceObserver.supportedEntryTypes?.includes('longtask')) {
		return { stop: () => undefined }
	}
	let total = 0
	const observer = new PerformanceObserver((list) => {
		for (const entry of list.getEntries()) total += entry.duration
	})
	observer.observe({ type: 'longtask' })
	return {
		stop: () => {
			for (const entry of observer.takeRecords()) total += entry.duration
			observer.disconnect()
			return total
		}
	}
}

/** Counts DOM nodes under `root`, shadow roots included. */
export function countNodes(root: ParentNode): number {
	let n = 0
	for (const el of Array.from(root.querySelectorAll('*'))) {
		n++
		if (el.shadowRoot) n += countNodes(el.shadowRoot)
	}
	return n
}

export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

export function formatMs(ms: number | undefined): string {
	return ms === undefined ? '–' : `${Math.round(ms)} ms`
}
