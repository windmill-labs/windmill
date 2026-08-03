import { getContext, setContext, untrack } from 'svelte'
import { randomUUID } from '$lib/utils/uuid'

export type OverlayStack = { val: string[] }

/** Overlays that answer to the viewport share this one. */
const globalOverlayStack: OverlayStack = $state({ val: [] })

/** Where overlays opened from this subtree (drawers, modals, popovers) belong. A host that
 *  embeds an editor in its own box provides one so its overlays stay inside that box — one
 *  tab's drawer must never cover a sibling tab or the chrome around it. */
export type OverlayHostContext = {
	/** Element the overlays anchor to instead of the viewport. */
	el: () => HTMLElement | undefined
	/** Stack this host arbitrates Escape and click-away on. */
	drawers: OverlayStack
	/** Whether this host is the one on screen. Hosts stay mounted while hidden, and
	 *  `opacity: 0` / `pointer-events: none` do not stop keyboard events from reaching a
	 *  `svelte:window` listener — so key handlers must ask before acting. */
	active: () => boolean
}

const OVERLAY_HOST_KEY = 'overlayHost'

export function setOverlayHost(host: OverlayHostContext) {
	setContext<OverlayHostContext>(OVERLAY_HOST_KEY, host)
}

export function getOverlayHost(): OverlayHostContext | undefined {
	return getContext<OverlayHostContext | undefined>(OVERLAY_HOST_KEY)
}

/**
 * The stack this component's overlays belong to. An enclosing pane keeps its own, because
 * it stays mounted while hidden: on the shared stack, an overlay left open in a hidden pane
 * would outrank — and swallow the Escape of — the pane the user is looking at.
 *
 * Reads context, so call it during component initialisation.
 */
export function overlayStack(): OverlayStack {
	return getOverlayHost()?.drawers ?? globalOverlayStack
}

/**
 * Portal target for an overlay: the host pane when one encloses this component, else the
 * given fallback.
 *
 * An id fallback such as `#flow-editor` is not necessarily unique — a session keeps every
 * visited tab mounted, so querySelector would drop the overlay into whichever editor came
 * first in the DOM. That tab is `opacity-0 pointer-events-none`, so the overlay renders
 * invisibly.
 *
 * Reads context, so call it during component initialisation; call the returned getter
 * where the target is used, to stay reactive as the host element mounts.
 */
export function overlayPortalTarget(fallback: string): () => HTMLElement | string {
	const host = getOverlayHost()
	return () => host?.el() ?? fallback
}

/**
 * Whether this component's overlays should respond to window-level keys. False only for
 * overlays living in a host that is currently hidden; true everywhere outside a host.
 */
export function overlayHostActive(): () => boolean {
	const host = getOverlayHost()
	return () => host?.active() ?? true
}

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
