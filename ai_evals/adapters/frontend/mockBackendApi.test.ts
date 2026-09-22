import { describe, expect, it } from 'bun:test'
import { handleBenchmarkApiFetch, hasBenchmarkApiHandler } from './mockBackend'

// `list_workers`'s client call is not faked, so its request really does leave as a relative
// `/api/...` url that node's fetch cannot resolve. Without this route the tool throws instead
// of answering.
describe('benchmark API fetch handlers', () => {
	it('lists workers, the way list_workers reaches them', async () => {
		const url = '/api/workers/list?per_page=100'

		expect(hasBenchmarkApiHandler(url)).toBe(true)
		const body = await handleBenchmarkApiFetch(url).json()

		// The tool counts the list and maps over it, so it needs a bare array, not an envelope.
		expect(Array.isArray(body)).toBe(true)
		expect(body[0]).toMatchObject({ worker: expect.any(String), worker_group: expect.any(String) })
	})
})
