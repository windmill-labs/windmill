import { tabbable } from 'tabbable'

/**
 * Build an Enter-key handler for `arrowTabNav`'s `onKeyDown` (or any
 * `keydown` listener): activate whatever is focused (so its `onClick`
 * fires), then move focus to the first tabbable inside the container
 * returned by `getNext`. preventDefault suppresses the browser's own
 * Enter→click so we don't double-fire.
 *
 *   {@attach arrowTabNav({ onKeyDown: selectAndAdvanceTo(() => nextEl) })}
 */
export function selectAndAdvanceTo(getNext: () => HTMLElement | undefined) {
	return (e: KeyboardEvent) => {
		if (e.key !== 'Enter') return
		const t = e.target as HTMLElement | null
		if (!t || t.tagName === 'TEXTAREA' || t.isContentEditable) return
		e.preventDefault()
		t.click()
		const next = getNext()
		if (next) tabbable(next)[0]?.focus()
	}
}
