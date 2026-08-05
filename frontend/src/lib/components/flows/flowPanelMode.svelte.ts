/** Below this editor width the step-details pane doesn't fit alongside the graph. */
const MODAL_PANEL_BREAKPOINT = 1280

/**
 * What the user asked for, which is not the same as where the panel ends up: `auto`
 * follows the editor's width, the other two pin it regardless of width.
 */
export type FlowPanelPreference = 'auto' | 'docked' | 'modal'

/** Where the panel actually is. */
type FlowPanelMode = 'docked' | 'modal'

/**
 * Resolves the step panel's position from the user's preference and the editor's own
 * width. The preference is not persisted: it lasts as long as the editor is open, so
 * every flow opens on `auto` and a pin is a deliberate act each time.
 */
export function useFlowPanelMode(opts: { enabled: () => boolean }) {
	let preference = $state<FlowPanelPreference>('auto')
	let width = $state(0)

	return {
		get preference(): FlowPanelPreference {
			return preference
		},
		set preference(next: FlowPanelPreference) {
			preference = next
		},
		get mode(): FlowPanelMode {
			if (!opts.enabled()) return 'docked'
			if (preference !== 'auto') return preference
			// A zero width means the editor has not been laid out yet. That is not evidence
			// of a narrow screen, so stay docked rather than flashing through a modal.
			return width > 0 && width < MODAL_PANEL_BREAKPOINT ? 'modal' : 'docked'
		},
		/** Fed by the editor root's measured width; drives `auto` in both directions. */
		measure(measured: number | null | undefined) {
			width = measured ?? 0
		}
	}
}
