import { describe, expect, it } from 'vitest'
import type { RoleCapabilities } from '$lib/gen'
import {
	capabilitiesIn,
	FULL_CAPABILITIES,
	hidesPages,
	isRestricted,
	withoutBuilding
} from './roleCapabilities'

const scripter: RoleCapabilities = {
	...FULL_CAPABILITIES,
	apps: 'none',
	visibility: { ...FULL_CAPABILITIES.visibility, workers: false }
}

describe('capabilitiesIn', () => {
	it("takes the server's capabilities, whatever the legacy operator flag says", () => {
		const caps = capabilitiesIn({ operator: false }, { capabilities: scripter })
		expect(caps).toEqual(scripter)
		expect(isRestricted(caps)).toBe(true)
	})

	it('gives an admin everything', () => {
		expect(capabilitiesIn({ is_admin: true }, { capabilities: scripter })).toBe(FULL_CAPABILITIES)
	})

	// A server that predates roles sends `operator` and `operator_settings` only.
	it('derives an operator from operator_settings on an older server', () => {
		const caps = capabilitiesIn(
			{ operator: true },
			{ operator_settings: { runs: true, builder_flows: true, manage_triggers: false } as any }
		)
		expect(caps.code).toBe(false)
		expect(caps.flows).toBe('compose')
		expect(caps.apps).toBe('none')
		expect(caps.manage_schedules).toBe(true)
		expect(caps.manage_triggers).toBe(false)
		expect(caps.visibility.runs).toBe(true)
		// `=== true`: a flag the settings never mention, or settings never written, hide the page.
		expect(caps.visibility.workers).toBe(false)
		expect(capabilitiesIn({ operator: true }, undefined).visibility.runs).toBe(false)
		expect(capabilitiesIn({ operator: false }, undefined)).toBe(FULL_CAPABILITIES)
	})

	// `RoleCapabilities::is_restricted` leaves visibility out, so a full author keeps forking.
	it('does not restrict a full role that only hides pages', () => {
		const hidden = {
			...FULL_CAPABILITIES,
			visibility: { ...FULL_CAPABILITIES.visibility, workers: false }
		}
		expect(isRestricted(hidden)).toBe(false)
		expect(hidesPages(hidden)).toBe(true)
	})
})

describe('withoutBuilding', () => {
	// Gates read "no code but may fork" as an operator granted forking, so Use mode drops both.
	it('takes forking away', () => {
		expect(withoutBuilding(FULL_CAPABILITIES).fork).toBe(false)
	})
})
