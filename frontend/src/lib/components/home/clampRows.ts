import type { Action } from 'svelte/action'

export type ClampRowsOptions = { rows: number; expanded: boolean }

/**
 * Keeps a wrapping row of `[data-chip]` items to `rows` lines, measured rather than counted, so
 * how many fit follows the width. The `[data-clamp-toggle]` child shows only on overflow, and
 * while collapsed it takes the end of the last line: items are hidden from there back until it
 * fits. A `[data-selected]` item is never hidden, since clicking it is how its filter clears.
 */
export const clampRows: Action<HTMLElement, ClampRowsOptions> = (node, initial) => {
	let options = initial
	let frame = 0
	let width = -1

	function run() {
		frame = 0
		const toggle = node.querySelector<HTMLElement>('[data-clamp-toggle]')
		const items = Array.from(node.querySelectorAll<HTMLElement>('[data-chip]'))
		for (const item of items) item.hidden = false
		if (toggle) toggle.hidden = true

		const lines = [...new Set(items.map((item) => item.offsetTop))].sort((a, b) => a - b)
		if (lines.length <= options.rows || !toggle) return
		toggle.hidden = false
		if (options.expanded) return

		const lastLine = lines[options.rows - 1]
		for (const item of items) {
			if (item.offsetTop > lastLine && !item.hasAttribute('data-selected')) item.hidden = true
		}
		const hideable = items.filter((item) => !item.hidden && !item.hasAttribute('data-selected'))
		while (toggle.offsetTop > lastLine && hideable.length) hideable.pop()!.hidden = true
	}

	function schedule() {
		if (!frame) frame = requestAnimationFrame(run)
	}

	// Width only: hiding items changes the height, which must not measure again.
	const resize = new ResizeObserver(([entry]) => {
		if (entry.contentRect.width === width) return
		width = entry.contentRect.width
		schedule()
	})
	resize.observe(node)
	// The items themselves changing; the `hidden` flips above are attributes, not seen here.
	const mutations = new MutationObserver(schedule)
	mutations.observe(node, { childList: true, subtree: true, characterData: true })

	return {
		update(next) {
			options = next
			schedule()
		},
		destroy() {
			cancelAnimationFrame(frame)
			resize.disconnect()
			mutations.disconnect()
		}
	}
}
