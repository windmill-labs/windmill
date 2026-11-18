import { describe, it, expect, beforeEach } from 'vitest'
import { rememberRerunOrigin, claimRerunOrigin } from './rerunResolution.svelte'

const origin = { originalId: 'failed', rerunId: 'rerun', workspace: 'w' }

// The pending origin lives at module scope, so each case starts by consuming whatever a
// previous one left behind.
beforeEach(() => claimRerunOrigin('rerun'))

describe('claimRerunOrigin', () => {
	it('claims only the run the re-run launched', () => {
		rememberRerunOrigin(origin)
		expect(claimRerunOrigin('rerun')).toEqual(origin)
	})

	// The offer resolves the original failure, so handing it to an unrelated run would mark a
	// failure handled on the strength of a success that has nothing to do with it.
	it('ignores an unrelated successful run and leaves the origin pending', () => {
		rememberRerunOrigin(origin)
		expect(claimRerunOrigin('someone-elses-run')).toBeUndefined()
		expect(claimRerunOrigin('rerun')).toEqual(origin)
	})

	it('claims once, so revisiting the re-run does not re-offer', () => {
		rememberRerunOrigin(origin)
		claimRerunOrigin('rerun')
		expect(claimRerunOrigin('rerun')).toBeUndefined()
	})
})
