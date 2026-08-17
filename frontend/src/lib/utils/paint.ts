import { tick } from 'svelte'

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
	await new Promise<void>((resolve) =>
		requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
	)
}
