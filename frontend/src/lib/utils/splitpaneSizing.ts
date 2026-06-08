/**
 * `svelte-splitpanes` only takes percentages, so convert a pixel `minSize`
 * threshold into a % of the container's current width (pair with
 * `bind:clientWidth`). Capped at `cap`%; returns 0 until the width is known.
 */
export function paneMinPercent(containerWidth: number, minPx: number, cap: number = 80): number {
	if (containerWidth <= 0) return 0
	return Math.min(cap, (minPx / containerWidth) * 100)
}
