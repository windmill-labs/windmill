/**
 * Helpers for using `svelte-splitpanes` with pixel-aware minimum widths.
 *
 * `svelte-splitpanes` only takes percentages for its `size`/`minSize`/`maxSize`
 * props, so when we need a pane to never shrink below a pixel threshold
 * (e.g. a sidebar that's unreadable under 160 px) we have to convert the
 * threshold into a percentage of the splitpane container's current width.
 *
 * Pattern at call sites:
 *
 * ```svelte
 * <div bind:clientWidth={containerWidth} class="h-full">
 *   <Splitpanes>
 *     <Pane minSize={paneMinPercent(containerWidth, 160)}>...</Pane>
 *   </Splitpanes>
 * </div>
 * ```
 */

/**
 * Returns the percentage of `containerWidth` that corresponds to `minPx`,
 * capped at `cap` percent (default 80) so callers can't accidentally make
 * the pane fill the container in very narrow viewports.
 *
 * Returns 0 when the container width isn't yet known (initial render).
 */
export function paneMinPercent(
	containerWidth: number,
	minPx: number,
	cap: number = 80
): number {
	if (containerWidth <= 0) return 0
	return Math.min(cap, (minPx / containerWidth) * 100)
}
