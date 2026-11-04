import { mount, unmount, type Snippet } from 'svelte'
import SnippetHost from './SnippetHost.svelte'

/**
 * Pierre's render callbacks return detached DOM nodes. This mounts a Svelte snippet into one
 * and keeps every mount so the owner can unmount them all when it is destroyed.
 */
export function snippetElements() {
	const mounted: Record<string, any>[] = []
	let destroyed = false
	return {
		render<T>(snippet: Snippet<[T]>, arg: T, tag: 'div' | 'span' = 'div'): HTMLElement {
			const target = document.createElement(tag)
			// Pierre re-renders synchronously when an edit session ends, which can be inside an
			// effect teardown, where Svelte refuses to create the mounted component's effects.
			queueMicrotask(() => {
				if (destroyed) return
				mounted.push(mount(SnippetHost<T>, { target, props: { snippet, arg } }))
			})
			return target
		},
		destroy() {
			destroyed = true
			for (const m of mounted.splice(0)) unmount(m)
		}
	}
}
