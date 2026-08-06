import { describe, expect, it } from 'vitest'
import { createBreakpointTracker, type FlowPanelPlacementEvent } from './flowEditorTelemetry'

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

	it('ignores a modal the user pinned', () => {
		const events: FlowPanelPlacementEvent[] = []
		const tracker = createBreakpointTracker((event) => events.push(event))

		tracker.observe('modal', 'modal')
		tracker.observe('docked', 'docked')

		expect(events).toEqual([])
	})
})
