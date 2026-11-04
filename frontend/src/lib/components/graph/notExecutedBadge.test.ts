import { describe, it, expect } from 'vitest'
import { notExecutedBadge } from './model'

const done = (skipped = false) => ({ type: 'Success' as const, args: {}, skipped })

describe('notExecutedBadge', () => {
	it('labels a mocked or skipped step only once it completed', () => {
		const mocked = { mock: { enabled: true } }
		expect(notExecutedBadge('a', mocked, done())?.label).toBe('Mocked')
		expect(notExecutedBadge('a', mocked, { type: 'InProgress', args: {} })).toBeUndefined()
		expect(notExecutedBadge('a', {}, done(true))?.label).toBe('Skipped')
	})

	it('does not trust the mock setting of an expanded subflow step', () => {
		expect(notExecutedBadge('subflow:s:a', { mock: { enabled: true } }, done())).toBeUndefined()
	})

	it('leaves a step that ran normally unmarked', () => {
		expect(notExecutedBadge('a', { mock: { enabled: false } }, done())).toBeUndefined()
	})
})
