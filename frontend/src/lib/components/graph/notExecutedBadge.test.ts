import { describe, it, expect } from 'vitest'
import { notExecutedBadge } from './model'

const done = (skipped = false) => ({ type: 'Success' as const, args: {}, skipped })

describe('notExecutedBadge', () => {
	it('labels a mocked or skipped step only once it completed', () => {
		const mocked = { mock: { enabled: true } }
		expect(notExecutedBadge(mocked, done())?.label).toBe('Mocked')
		expect(notExecutedBadge(mocked, { type: 'InProgress', args: {} })).toBeUndefined()
		expect(notExecutedBadge({}, done(true))?.label).toBe('Skipped')
	})

	it('leaves a step that ran normally unmarked', () => {
		expect(notExecutedBadge({ mock: { enabled: false } }, done())).toBeUndefined()
	})
})
