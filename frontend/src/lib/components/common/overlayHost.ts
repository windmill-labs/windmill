import { getContext, setContext } from 'svelte'

/** Element that overlays opened from this subtree (drawers, the flow editor's
 *  detached panel) anchor to instead of the viewport. A host that embeds an editor
 *  in its own box provides one so its overlays stay inside that box — one tab's
 *  drawer must never cover a sibling tab or the chrome around it. */
export type OverlayHostContext = () => HTMLElement | undefined

const OVERLAY_HOST_KEY = 'overlayHost'

export function setOverlayHost(host: OverlayHostContext) {
	setContext<OverlayHostContext>(OVERLAY_HOST_KEY, host)
}

export function getOverlayHost(): OverlayHostContext | undefined {
	return getContext<OverlayHostContext | undefined>(OVERLAY_HOST_KEY)
}
