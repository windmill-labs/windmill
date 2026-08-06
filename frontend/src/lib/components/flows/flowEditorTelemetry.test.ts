import { describe, expect, it } from 'vitest'
import {
	createBreakpointTracker,
	forcedPlacementEvent,
	type FlowPanelPlacementEvent
} from './flowEditorTelemetry'

describe('createBreakpointTracker', () => {
	it('counts a crossing into modal once, however many times the width settles', () => {
		const events: FlowPanelPlacementEvent[] = []
		const tracker = createBreakpointTracker((event) => events.push(event))

		tracker.observe('auto', 'docked')
		tracker.observe('auto', 'modal')
		// A window drag re-resolves the mode continuously; the breakpoint activated once.
		tracker.observe('auto', 'modal')
		tracker.observe('auto', 'docked')
		tracker.observe('auto', 'modal')

		expect(events).toEqual(['breakpoint_modal', 'breakpoint_modal'])
	})

	it('does not re-count when a pin is released back to auto without moving the panel', () => {
		const events: FlowPanelPlacementEvent[] = []
		const tracker = createBreakpointTracker((event) => events.push(event))

		tracker.observe('auto', 'modal')
		// Pinning Detached on a narrow editor, then releasing it, leaves the panel modal
		// throughout — re-arming on the preference alone would read that as a second crossing.
		tracker.observe('modal', 'modal')
		tracker.observe('auto', 'modal')

		expect(events).toEqual(['breakpoint_modal'])
	})

	it('ignores a modal the user pinned', () => {
		const events: FlowPanelPlacementEvent[] = []
		const tracker = createBreakpointTracker((event) => events.push(event))

		tracker.observe('modal', 'modal')
		tracker.observe('docked', 'docked')

		expect(events).toEqual([])
	})
})

describe('forcedPlacementEvent', () => {
	it('reports only the pins that move the panel', () => {
		expect(forcedPlacementEvent('docked', 'modal')).toBe('force_attach')
		expect(forcedPlacementEvent('modal', 'docked')).toBe('force_detach')
		// Already where it is pinned to: a stated preference, not an override.
		expect(forcedPlacementEvent('docked', 'docked')).toBeUndefined()
		expect(forcedPlacementEvent('modal', 'modal')).toBeUndefined()
		expect(forcedPlacementEvent('auto', 'modal')).toBeUndefined()
		expect(forcedPlacementEvent('auto', 'docked')).toBeUndefined()
	})
})
