import { logFeatureUsage } from '$lib/utils/featureUsage'
import type { FlowPanelMode, FlowPanelPreference } from './panelPlacement'

// Anonymous counters for where the flow editor's step panel ends up. Same rules as every
// other `logFeatureUsage` caller: aggregated counts only, and the three keys below are the
// whole vocabulary — no path, expression or step id ever reaches here.

const FEATURE = 'flow_editor'
const KIND = 'panel_placement'

export type FlowPanelPlacementEvent =
	/** `auto` resolved to modal because the editor is narrower than the breakpoint. */
	| 'breakpoint_modal'
	/** The user pinned the panel into the pane, overriding `auto`. */
	| 'force_attach'
	/** The user pinned the panel out into the modal, overriding `auto`. */
	| 'force_detach'

export function logPanelPlacement(event: FlowPanelPlacementEvent): void {
	logFeatureUsage(FEATURE, KIND, { key: event })
}

/** Returns nothing for `auto`: going back to automatic is not a placement being forced. */
export function forcedPlacementEvent(
	preference: FlowPanelPreference
): FlowPanelPlacementEvent | undefined {
	if (preference === 'docked') return 'force_attach'
	if (preference === 'modal') return 'force_detach'
	return undefined
}

/**
 * Counts the breakpoint taking the panel into the modal, once per crossing. Free of runes so
 * the edge can be tested without a component: `mode` is derived from a measured width, so
 * anything reading it per evaluation rather than per transition would count a window drag as
 * hundreds of activations.
 */
export function createBreakpointTracker(log: (event: FlowPanelPlacementEvent) => void) {
	let active = false

	return {
		observe(preference: FlowPanelPreference, mode: FlowPanelMode) {
			const next = preference === 'auto' && mode === 'modal'
			if (next && !active) log('breakpoint_modal')
			active = next
		}
	}
}
