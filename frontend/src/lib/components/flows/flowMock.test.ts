import { describe, expect, it } from 'vitest'
import { nextMock } from './flowMock'

describe('nextMock', () => {
	it('limits a first pin to test runs and keeps a legacy pin on every run', () => {
		expect(nextMock(undefined, { enabled: true }).preview_only).toBe(true)
		expect(nextMock({ enabled: false }, { enabled: true }).preview_only).toBeUndefined()
	})
})
