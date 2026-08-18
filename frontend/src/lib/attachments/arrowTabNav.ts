import { tabbable } from 'tabbable'
import type { Attachment } from 'svelte/attachments'

export interface ArrowTabNavOptions {
	/** Which arrow-key pair walks the tab order. Default `'y'` (Up/Down). */
	axis?: 'x' | 'y'
	/** Use this to layer custom keys (Enter, Escape …) */
	onKeyDown?: (e: KeyboardEvent) => void
}

/**
 * Map one axis of arrow keys to next/previous in Tab order, scoped to
 * the attached element. Wraps around at the ends. Bails when the
 * keypress originates inside a text input / textarea / contenteditable
 * so the caret can still move with the arrows.
 *
 *   <div {@attach arrowTabNav()}>…</div>
 *   <div {@attach arrowTabNav({ axis: 'x' })}>…</div>
 *   <div {@attach arrowTabNav({ onKeyDown: (e) => { … } })}>…</div>
 */
export function arrowTabNav(opts: ArrowTabNavOptions = {}): Attachment<HTMLElement> {
	const axis = opts.axis ?? 'y'
	const nextKey = axis === 'y' ? 'ArrowDown' : 'ArrowRight'
	const prevKey = axis === 'y' ? 'ArrowUp' : 'ArrowLeft'

	return (node) => {
		const handler = (e: KeyboardEvent) => {
			// Run the consumer's handler first so they can preventDefault
			// or do their own thing before we react to arrows.
			opts.onKeyDown?.(e)

			if (e.key !== nextKey && e.key !== prevKey) return

			// Let editable fields keep their native caret behavior.
			const t = e.target as HTMLElement | null
			if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) {
				return
			}

			const items = tabbable(node)
			if (items.length === 0) return

			const active = document.activeElement as HTMLElement | null
			const i = active ? items.indexOf(active) : -1
			const dir = e.key === nextKey ? 1 : -1
			// No tabbable currently focused: jump to the first (next) or last (prev).
			const next =
				i === -1
					? items[dir === 1 ? 0 : items.length - 1]
					: items[(i + dir + items.length) % items.length]

			e.preventDefault()
			next?.focus()
		}

		node.addEventListener('keydown', handler)
		return () => node.removeEventListener('keydown', handler)
	}
}
