import { untrack } from 'svelte'
import { randomUUID } from '$lib/utils/uuid'
import { overlayHostActive, overlayStack } from '$lib/components/common/overlayHost.svelte'

/**
 * Take a place on this subtree's overlay stack while open, and report whether this overlay
 * is the topmost one.
 *
 * Escape belongs to whatever is on top, and every handler involved listens on `window`, so
 * none can stop another — the stack is the only arbiter. Joining it is what makes the
 * ordering work in both directions: a drawer opened from inside this overlay outranks it,
 * while the drawer this overlay is nested in does not.
 */
export function useOverlayStack(isOpen: () => boolean) {
	const id = randomUUID()
	const openedDrawers = overlayStack()
	const hostActive = overlayHostActive()

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
		// The stack is per-host, so an overlay alone in a hidden pane is topmost of its own
		// stack. Its `svelte:window` handler still fires, so being on screen is part of the
		// question — without this it would answer Escape for the pane the user is looking at.
		isTopmost: () => hostActive() && openedDrawers.val.at(-1) === id
	}
}
