import { describe, expect, it } from 'vitest'
import { classicAppDraftValue } from './appDiffSides'

describe('classicAppDraftValue', () => {
	it('keeps a bare grid draft as-is (minus parent_version)', () => {
		const grid = { grid: [{ id: 'a' }], fullscreen: false, parent_version: 7 }
		expect(classicAppDraftValue(grid)).toEqual({ grid: [{ id: 'a' }], fullscreen: false })
	})

	it('unwraps a legacy wrapped draft so it compares against the deployed value', () => {
		const wrapped = {
			summary: 'My app',
			policy: {},
			value: { grid: [{ id: 'a' }] }
		}
		expect(classicAppDraftValue(wrapped)).toEqual({ grid: [{ id: 'a' }] })
	})

	it('does not mistake a grid whose component is named value for a wrapper', () => {
		const grid = { grid: [], value: { some: 'component-state' } }
		expect(classicAppDraftValue(grid)).toEqual(grid)
	})
})
