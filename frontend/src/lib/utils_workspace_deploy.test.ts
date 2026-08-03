import { describe, it, expect } from 'vitest'
import {
	diffActionableInDirection,
	diffCreatesInTarget,
	diffRemovesInTarget
} from './utils_workspace_deploy'

/** The row shape the fork comparison returns for an item the parent has and the
 * fork does not. `ahead = 1, behind = 0` is what the tally leaves after the fork
 * pulled the parent's item in and then lost it (a delete, a git-sync revert):
 * every deploy event in the fork counts as `ahead`, whatever wrote it. */
const parentOnly = { ahead: 1, behind: 0, exists_in_source: true, exists_in_fork: false }
const forkOnly = { ahead: 1, behind: 0, exists_in_source: false, exists_in_fork: true }
const bothSides = { ahead: 1, behind: 1, exists_in_source: true, exists_in_fork: true }

describe('deploy direction of a one-sided diff row', () => {
	it('offers a parent-only item to the fork even with no behind count', () => {
		expect(diffActionableInDirection(parentOnly, false)).toBe(true)
		expect(diffCreatesInTarget(parentOnly, false)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, false)).toBe(false)
	})

	it('keeps a parent-only row out of a merge into the parent, whatever its ahead count', () => {
		expect(diffActionableInDirection(parentOnly, true)).toBe(false)
		// An arbitrary target has no tally behind it and does propagate the removal.
		expect(diffActionableInDirection(parentOnly, true, true)).toBe(true)
		expect(diffRemovesInTarget(parentOnly, true)).toBe(true)
	})

	it('does not resurrect a fork-only item into an update of the fork', () => {
		expect(diffActionableInDirection(forkOnly, false)).toBe(false)
		expect(diffCreatesInTarget(forkOnly, true)).toBe(true)
	})

	it('keeps a two-sided row on its counters', () => {
		expect(diffActionableInDirection(bothSides, true)).toBe(true)
		expect(diffActionableInDirection({ ...bothSides, behind: 0 }, false)).toBe(false)
		expect(diffCreatesInTarget(bothSides, true)).toBe(false)
		expect(diffRemovesInTarget(bothSides, false)).toBe(false)
	})
})
