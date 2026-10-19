import { describe, expect, it } from 'vitest'
import { composerCollapses } from './composerCollapse'

const atRest = { onHome: true, usable: true, fewItems: false, opened: false, draftEmpty: true }

describe('composerCollapses', () => {
	it('collapses only on Home, at rest, in a workspace that is not small', () => {
		expect(composerCollapses(atRest)).toBe(true)
		expect(composerCollapses({ ...atRest, onHome: false })).toBe(false)
		expect(composerCollapses({ ...atRest, fewItems: true })).toBe(false)
		expect(composerCollapses({ ...atRest, usable: false })).toBe(false)
	})

	it('stays open while the user is in it or has a draft', () => {
		expect(composerCollapses({ ...atRest, opened: true })).toBe(false)
		expect(composerCollapses({ ...atRest, draftEmpty: false })).toBe(false)
	})
})
