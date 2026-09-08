import { describe, it, expect } from 'vitest'
import { graphParticipates, nextArmed, type ArmedTarget } from './connectPolicy'

const target = (id: string): ArmedTarget => ({ id, onSelect: () => {} })

describe('nextArmed', () => {
	it('replaces the previous target so a pick has one destination', () => {
		expect(nextArmed(target('iterator'), target('region'))?.id).toBe('region')
	})

	it('disarms when the already-armed input is clicked again', () => {
		expect(nextArmed(target('region'), target('region'))).toBeUndefined()
	})
})

describe('graphParticipates', () => {
	const docked = { inModalPanel: false, hasPickableProperties: true }

	it('offers the graph while a target is armed in a docked panel', () => {
		expect(graphParticipates(target('region'), docked)).toBe(true)
	})

	it('never offers the graph from a modal panel, which covers it', () => {
		expect(graphParticipates(target('region'), { ...docked, inModalPanel: true })).toBe(false)
	})

	it('stays out of it when the input has nothing to pick from', () => {
		// Sleep and suspend show previous results but expose no pickable properties.
		expect(graphParticipates(target('sleep'), { ...docked, hasPickableProperties: false })).toBe(
			false
		)
	})

	it('is off whenever nothing is armed', () => {
		expect(graphParticipates(undefined, docked)).toBe(false)
	})
})
