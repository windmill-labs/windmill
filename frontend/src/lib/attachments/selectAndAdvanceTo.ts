import { tabbable } from 'tabbable'

/**
 * Build an Enter-key handler for `arrowTabNav`'s `onKeyDown` (or any
 * `keydown` listener): activate whatever is focused (so its `onClick`
 * fires), then move focus to the first tabbable inside the container
 * returned by `getNext`. preventDefault suppresses the browser's own
 * Enter→click so we don't double-fire.
 *
 * By default the handler bails inside `<textarea>` / contenteditable so
 * Enter keeps inserting a newline there. Pass `{ requireModifier: true }`
 * on textarea-bearing steps — then bare Enter still inserts a newline
 * and only Cmd/Ctrl+Enter triggers the advance.
 *
 * Pass `{ timeout: <ms> }` when the next container only renders as a
 * consequence of the click (e.g. a bottom panel gated on the freshly
 * set selection): both the `getNext()` lookup and the focus shift run
 * after the delay, giving the framework time to flush its render.
 *
 *   {@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => nextEl) })}
 */
export function selectAndAdvanceTo(
	getNext: () => HTMLElement | undefined,
	opts: { requireModifier?: boolean; timeout?: number } = {}
) {
	return (e: KeyboardEvent) => {
		if (e.key !== 'Enter') return
		const t = e.target as HTMLElement | null
		if (!t) return
		if (opts.requireModifier) {
			if (!e.metaKey && !e.ctrlKey) return
		} else if (t.tagName === 'TEXTAREA' || t.isContentEditable) {
			return
		}
		e.preventDefault()
		e.stopPropagation()
		t.click()

		const advance = () => {
			const next = getNext()
			if (next) tabbable(next)[0]?.focus()
		}
		if (opts.timeout && opts.timeout > 0) {
			setTimeout(advance, opts.timeout)
		} else {
			advance()
		}
	}
}
