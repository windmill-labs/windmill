import { mount, unmount, type Snippet } from 'svelte'
import SnippetHost from './SnippetHost.svelte'

type Entry = { target: HTMLElement; component?: Record<string, any>; seen: boolean; frames: number }

// Pierre may build an element and drop it in the same pass without inserting it.
const MAX_UNSEEN_FRAMES = 10

/**
 * Pierre's render callbacks return detached DOM nodes and later remove them without telling
 * the caller. This mounts a Svelte snippet into each one and unmounts it once its node has
 * left the document, checked on the frame after a render so a node Pierre detaches and
 * reinserts within one render is kept.
 */
export function snippetElements() {
	const entries = new Set<Entry>()
	let destroyed = false
	let sweepScheduled = false

	function drop(entry: Entry) {
		if (entry.component) unmount(entry.component)
		entries.delete(entry)
	}

	function sweep() {
		sweepScheduled = false
		if (destroyed) return
		let pending = false
		for (const entry of entries) {
			if (entry.target.isConnected) {
				entry.seen = true
			} else if (entry.seen || ++entry.frames > MAX_UNSEEN_FRAMES) {
				drop(entry)
			} else {
				pending = true
			}
		}
		if (pending) scheduleSweep()
	}

	function scheduleSweep() {
		if (sweepScheduled) return
		sweepScheduled = true
		requestAnimationFrame(sweep)
	}

	return {
		render<T>(snippet: Snippet<[T]>, arg: T, tag: 'div' | 'span' = 'div'): HTMLElement {
			const entry: Entry = { target: document.createElement(tag), seen: false, frames: 0 }
			entries.add(entry)
			// Pierre re-renders synchronously when an edit session ends, which can be inside an
			// effect teardown, where Svelte refuses to create the mounted component's effects.
			queueMicrotask(() => {
				if (destroyed || !entries.has(entry)) return
				entry.component = mount(SnippetHost<T>, { target: entry.target, props: { snippet, arg } })
			})
			scheduleSweep()
			return entry.target
		},
		destroy() {
			destroyed = true
			for (const entry of entries) drop(entry)
		}
	}
}
