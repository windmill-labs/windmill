import { getContext, setContext } from 'svelte'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import type { FlowPanelMode, FlowPanelPreference } from './panelPlacement'

// Anonymous counters for where the flow editor's step panel ends up. Same rules as every
// other `logFeatureUsage` caller: aggregated counts only, and the three keys below are the
// whole vocabulary — no path, expression or step id ever reaches here.

const FEATURE = 'flow_editor'
const KIND = 'panel_placement'

export type FlowPanelPlacementEvent =
	/** The width moved the panel into the modal, under `auto`. */
	| 'breakpoint_modal'
	/** The user pinned the panel into the pane while it was in the modal. */
	| 'force_attach'
	/** The user pinned the panel out into the modal while it was in the pane. */
	| 'force_detach'

type Log = (event: FlowPanelPlacementEvent) => void

const log: Log = (event) => logFeatureUsage(FEATURE, KIND, { key: event })

/**
 * The event a placement pin should produce, or nothing. `auto` is not a placement being
 * forced, and a pin that matches where the panel already is moves nothing — the counters
 * carry no width, so counting that would be indistinguishable from the override that did
 * move the panel.
 */
export function forcedPlacementEvent(
	preference: FlowPanelPreference,
	mode: FlowPanelMode
): FlowPanelPlacementEvent | undefined {
	if (preference === mode) return undefined
	if (preference === 'docked') return 'force_attach'
	if (preference === 'modal') return 'force_detach'
	return undefined
}

/**
 * Counts the width moving the panel into the modal.
 *
 * Tracks where the panel was, not whether the breakpoint was responsible for it being
 * there: on a narrow editor, pinning `Detached` and returning to `auto` leaves the panel
 * modal throughout, and re-arming on the preference alone would read that as a second
 * activation. An unmeasured editor is not a placement at all — it resolves to `docked`
 * because that is what is safe to render — so it must leave the tracker untouched rather
 * than count as the panel having docked. Free of runes so every edge can be tested without
 * a component: `mode` is derived from a measured width, so anything counting per evaluation
 * rather than per transition would read one window drag as hundreds of activations.
 */
export function createBreakpointTracker(emit: Log) {
	let wasModal = false

	return {
		observe(preference: FlowPanelPreference, mode: FlowPanelMode, measured: boolean) {
			if (!measured) return
			if (mode === 'modal' && !wasModal && preference === 'auto') emit('breakpoint_modal')
			wasModal = mode === 'modal'
		}
	}
}

export interface FlowPanelPlacementTelemetry {
	/** The panel's current placement; emits `breakpoint_modal` on a crossing into the modal. */
	observe(preference: FlowPanelPreference, mode: FlowPanelMode, measured: boolean): void
	/** A placement the user pinned, against where the panel was when they pinned it. */
	forced(preference: FlowPanelPreference, mode: FlowPanelMode): void
}

const CONTEXT_KEY = 'flowPanelPlacementTelemetry'

const NOOP: FlowPanelPlacementTelemetry = { observe: () => {}, forced: () => {} }

/**
 * Published by `FlowBuilder`, which sits above the `{#key}` that rebuilds the editor on a
 * reload: crossings belong to the editing session, and a tracker recreated mid-edit would
 * re-arm and count a still-narrow editor again without the panel having moved. Surviving
 * the remount is only half of it — the rebuilt editor measures zero before its width
 * arrives, which is why the tracker ignores unmeasured placements instead of reading that
 * as a dock.
 *
 * `enabled` is false in session preview tabs. Those stay mounted and laid out at panel width
 * even while hidden, and that panel is narrower than the breakpoint by construction: their
 * crossings would bury the ones this measures, and their overrides would then be read
 * against a denominator that no longer contains them.
 */
export function setFlowPanelPlacementTelemetry(enabled: boolean): void {
	const emit: Log = enabled ? log : () => {}
	const tracker = createBreakpointTracker(emit)
	setContext<FlowPanelPlacementTelemetry>(CONTEXT_KEY, {
		observe: tracker.observe,
		forced: (preference, mode) => {
			const event = forcedPlacementEvent(preference, mode)
			if (event) emit(event)
		}
	})
}

export function useFlowPanelPlacementTelemetry(): FlowPanelPlacementTelemetry {
	return getContext<FlowPanelPlacementTelemetry | undefined>(CONTEXT_KEY) ?? NOOP
}
