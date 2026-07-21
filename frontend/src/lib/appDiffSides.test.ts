import { describe, expect, it } from 'vitest'
import { classicAppDraftParts } from './appDiffSides'

describe('classicAppDraftParts', () => {
	it('extracts the mirrored summary and strips draft-only markers from the grid', () => {
		const draft = {
			grid: [{ id: 'a' }],
			fullscreen: false,
			summary: 'My app',
			draft_path: 'f/nice/name',
			parent_version: 7
		}
		expect(classicAppDraftParts(draft)).toEqual({
			value: { grid: [{ id: 'a' }], fullscreen: false },
			summary: 'My app'
		})
	})

	it('unwraps a legacy wrapped draft so it compares against the deployed value', () => {
		const wrapped = {
			summary: 'My app',
			policy: {},
			value: { grid: [{ id: 'a' }] }
		}
		expect(classicAppDraftParts(wrapped)).toEqual({
			value: { grid: [{ id: 'a' }] },
			summary: 'My app'
		})
	})

	it('does not mistake a grid whose component is named value for a wrapper', () => {
		const grid = { grid: [], value: { some: 'component-state' } }
		expect(classicAppDraftParts(grid)).toEqual({ value: grid, summary: undefined })
	})
})
