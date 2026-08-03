import { getContext, setContext } from 'svelte'

export type OverlayStack = { val: string[] }

/** Overlays that answer to the viewport share this one. */
const globalOverlayStack: OverlayStack = $state({ val: [] })

/** Where overlays opened from this subtree (drawers, modals, the flow editor's detached
 *  panel) belong. A host that embeds an editor in its own box provides one so its overlays
 *  stay inside that box — one tab's drawer must never cover a sibling tab or the chrome
 *  around it. */
export type OverlayHostContext = {
	/** Element the overlays anchor to instead of the viewport. */
	el: () => HTMLElement | undefined
	/** Stack this host arbitrates Escape and click-away on. */
	drawers: OverlayStack
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
 * it stays mounted while hidden: on the shared stack, an overlay left open in a hidden tab
 * would outrank — and swallow the Escape of — the tab the user is looking at.
 *
 * Reads context, so call it during component initialisation.
 */
export function overlayStack(): OverlayStack {
	return getOverlayHost()?.drawers ?? globalOverlayStack
}

/**
 * Portal target for an overlay that belongs to a flow editor: the host pane when the
 * editor is embedded in one, else the editor root.
 *
 * `#flow-editor` is not a unique id — a session keeps every visited tab mounted, so
 * querySelector would drop the overlay into whichever editor came first in the DOM.
 * That tab is `opacity-0 pointer-events-none`, so the overlay renders invisibly.
 *
 * Reads context, so call it during component initialisation; call the returned getter
 * where the target is used, to stay reactive as the host element mounts.
 */
export function overlayPortalTarget(fallback: string): () => HTMLElement | string {
	const host = getOverlayHost()
	return () => host?.el() ?? fallback
}
