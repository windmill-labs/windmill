import { describe, expect, it } from 'vitest'

import { latestRequest } from './latestRequest'

describe('latestRequest', () => {
	// The trigger status refresh peeks so it can never drop a full list's reply.
	it('start supersedes earlier requests; peek supersedes nothing', () => {
		const loads = latestRequest()
		const load = loads.start()
		const peeked = loads.peek()
		expect(load()).toBe(true)
		expect(peeked()).toBe(true)
		loads.start()
		expect(load()).toBe(false)
		expect(peeked()).toBe(false)
	})
})
