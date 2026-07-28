import { untrack } from 'svelte'
import { randomUUID } from '$lib/utils/uuid'

/**
 * Take a place on the shared overlay stack while open, and report whether this overlay is
 * the topmost one.
 *
 * Escape belongs to whatever is on top, and every handler involved listens on `window`, so
 * none can stop another — the stack is the only arbiter. Joining it is what makes the
 * ordering work in both directions: a drawer opened from inside this overlay outranks it,
 * while the drawer this overlay is nested in does not.
 */
export function useOverlayStack(isOpen: () => boolean, openedDrawers: { val: string[] }) {
	const id = randomUUID()

	$effect(() => {
		if (isOpen()) {
			// untrack: `push` reads `length` through the state proxy, so a tracked write here
			// marks this very effect dirty and re-runs it until Svelte's loop guard throws.
			untrack(() => openedDrawers.val.push(id))
			return () => {
				openedDrawers.val = openedDrawers.val.filter((d) => d !== id)
			}
		}
	})

	return {
		isTopmost: () => openedDrawers.val.at(-1) === id
	}
}
