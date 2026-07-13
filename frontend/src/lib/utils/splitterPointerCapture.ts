/**
 * svelte-splitpanes tracks drags with document-level mousemove/mouseup only.
 * A button release outside the browser window never reaches the document, so
 * the drag state sticks: global col-resize cursor and pointer-events:none on
 * every pane until the next click. Capturing the pointer on the splitter at
 * pointerdown makes the browser deliver the release (and its compatibility
 * mouseup, which bubbles to the library's document listener) even when it
 * happens outside the window. Attach to any ancestor of the Splitpanes.
 */
export function splitterPointerCapture(node: HTMLElement) {
	function onPointerDown(e: PointerEvent) {
		const splitter = (e.target as Element | null)?.closest?.('.splitpanes__splitter')
		if (splitter instanceof HTMLElement) {
			try {
				splitter.setPointerCapture(e.pointerId)
			} catch {
				// Non-capturable pointer (already released) — the plain document
				// listeners still handle the in-window case.
			}
		}
	}
	node.addEventListener('pointerdown', onPointerDown, true)
	return { destroy: () => node.removeEventListener('pointerdown', onPointerDown, true) }
}
