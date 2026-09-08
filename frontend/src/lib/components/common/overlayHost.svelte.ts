import { getContext, setContext } from 'svelte'

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
 * Portal target for an overlay: the caller's own `fallback` container, resolved within the
 * enclosing host pane when there is one.
 *
 * An id fallback such as `#flow-editor` is not unique — a session keeps every visited tab
 * mounted, so a document-wide querySelector would drop the overlay into whichever editor
 * came first in the DOM. That tab is `opacity-0 pointer-events-none`, so the overlay
 * renders invisibly. Scoping the lookup to the host keeps the caller's chosen container
 * (which sets the overlay's offset parent and clipping) while picking this tab's copy of
 * it; a fallback that names nothing inside the host, such as `body`, falls back to the
 * host itself.
 *
 * Reads context, so call it during component initialisation; call the returned getter
 * where the target is used, to stay reactive as the host element mounts.
 */
export function overlayPortalTarget(fallback: string | (() => string)): () => HTMLElement | string {
	const host = getOverlayHost()
	return () => {
		const selector = typeof fallback === 'function' ? fallback() : fallback
		const el = host?.el()
		if (!el) return selector
		return el.querySelector<HTMLElement>(selector) ?? el
	}
}

/**
 * Whether this component's overlays should respond to window-level keys. False only for
 * overlays living in a host that is currently hidden; true everywhere outside a host.
 */
export function overlayHostActive(): () => boolean {
	const host = getOverlayHost()
	return () => host?.active() ?? true
}

const TOPMOST_SURFACE_KEY = 'topmostSurface'

/**
 * Declare whether the surface enclosing this subtree is the one on top. Set by whatever owns the
 * stacking — a dialog, a drawer — so content inside it can tell a key meant for itself from one
 * meant for something opened over it.
 */
export function setTopmostSurface(isTopmost: () => boolean) {
	setContext(TOPMOST_SURFACE_KEY, isTopmost)
}

/**
 * Whether the enclosing surface is on top. True when nothing declared otherwise, so content that
 * is not inside such a surface is not silently made deaf.
 */
export function topmostSurface(): () => boolean {
	const isTopmost = getContext<(() => boolean) | undefined>(TOPMOST_SURFACE_KEY)
	return () => isTopmost?.() ?? true
}
