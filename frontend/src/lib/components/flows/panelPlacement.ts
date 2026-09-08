/** Below this editor width the step-details pane doesn't fit alongside the graph. */
export const MODAL_PANEL_BREAKPOINT = 1280

/**
 * What the user asked for, which is not the same as where the panel ends up: `auto`
 * follows the editor's width, the other two pin it regardless of width.
 */
export type FlowPanelPreference = 'auto' | 'docked' | 'modal'

/** Where the panel actually is. */
export type FlowPanelMode = 'docked' | 'modal'

/**
 * Resolves where the step panel goes. Split from the controller that holds the state so
 * the rule can be tested without a component: the zero-width case below is the one a
 * refactor gets wrong silently, since an unlaid-out editor and a narrow one both measure
 * small and only one of them should detach.
 */
export function resolvePanelMode(input: {
	enabled: boolean
	preference: FlowPanelPreference
	width: number
}): FlowPanelMode {
	if (!input.enabled) return 'docked'
	if (input.preference !== 'auto') return input.preference
	// A zero width means the editor has not been laid out yet. That is not evidence of a
	// narrow screen, so stay docked rather than flashing through a modal.
	return input.width > 0 && input.width < MODAL_PANEL_BREAKPOINT ? 'modal' : 'docked'
}
