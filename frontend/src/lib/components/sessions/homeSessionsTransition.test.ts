import { describe, expect, it } from 'vitest'
import { composerFocusAfter } from './homeSessionsTransition.svelte'

describe('composerFocusAfter', () => {
	it('hands focus on only for the return from AI Sessions to Home', () => {
		expect(composerFocusAfter('/sessions', '/', true)).toBe(true)
		expect(composerFocusAfter('/sessions', '/', false)).toBe(false)
		// Going to AI Sessions: that page focuses its own composer, so nothing is kept for Home.
		expect(composerFocusAfter('/', '/sessions', true)).toBe(false)
		// Any other navigation clears it, so a later arrival on Home never reads a stale flag.
		expect(composerFocusAfter('/sessions', '/runs', true)).toBe(false)
		expect(composerFocusAfter('/runs', '/', true)).toBe(false)
	})
})
