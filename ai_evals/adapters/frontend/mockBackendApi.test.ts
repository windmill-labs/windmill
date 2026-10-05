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

// A benchmark whose list calls an integration undocumented while its metadata endpoint
// hands back authored notes teaches the model the flag means nothing.
describe('benchmark hub integration list', () => {
	it('flags exactly the integrations whose metadata carries authored notes', async () => {
		const listed = (await handleBenchmarkApiFetch('/api/integrations/hub/list').json()) as Array<{
			name: string
			documented: boolean
		}>
		expect(listed.length).toBeGreaterThan(0)

		for (const { name, documented } of listed) {
			const res = handleBenchmarkApiFetch(`/api/integrations/hub/${name}/meta`)
			const authored = res.status === 200 && !!((await res.json()) as { meta?: unknown }).meta
			expect(authored).toBe(documented)
		}
	})
})
