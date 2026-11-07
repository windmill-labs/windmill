import { mount, unmount, type Snippet } from 'svelte'
import SnippetHost from './SnippetHost.svelte'

type Entry = { target: HTMLElement; component?: Record<string, any>; seen: boolean; sweeps: number }

// Pierre may build an element and drop it in the same pass without inserting it.
const MAX_UNSEEN_SWEEPS = 10
const SWEEP_MS = 500

/**
 * Pierre's render callbacks return detached DOM nodes and later remove them without telling
 * the caller (a re-render with fewer annotations, a selection popover closing). This mounts a
 * Svelte snippet into each one and unmounts it once its node has left the document. Nodes are
 * checked on a timer rather than at render, so one Pierre detaches and reinserts within a
 * render is kept, and one removed with no render following is still released.
 */
export function snippetElements() {
	const entries = new Set<Entry>()
	let timer: ReturnType<typeof setInterval> | undefined

	function drop(entry: Entry) {
		if (entry.component) unmount(entry.component)
		entries.delete(entry)
	}

	function sweep() {
		for (const entry of entries) {
			if (entry.target.isConnected) entry.seen = true
			else if (entry.seen || ++entry.sweeps > MAX_UNSEEN_SWEEPS) drop(entry)
		}
		if (entries.size === 0) stop()
	}

	function stop() {
		clearInterval(timer)
		timer = undefined
	}

	return {
		render<T>(snippet: Snippet<[T]>, arg: T, tag: 'div' | 'span' = 'div'): HTMLElement {
			const entry: Entry = { target: document.createElement(tag), seen: false, sweeps: 0 }
			entries.add(entry)
			timer ??= setInterval(sweep, SWEEP_MS)
			// Pierre re-renders synchronously when an edit session ends, which can be inside an
			// effect teardown, where Svelte refuses to create the mounted component's effects.
			queueMicrotask(() => {
				if (!entries.has(entry)) return
				entry.component = mount(SnippetHost<T>, { target: entry.target, props: { snippet, arg } })
			})
			return entry.target
		},
		destroy() {
			stop()
			for (const entry of entries) drop(entry)
		}
	}
}
