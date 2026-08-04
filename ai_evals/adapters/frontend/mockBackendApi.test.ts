import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import {
	createBenchmarkCompletedJob,
	handleBenchmarkApiFetch,
	hasBenchmarkApiHandler,
	listBenchmarkMcpTools,
	resetBenchmarkMockBackend
} from './mockBackend'

const WORKSPACE = 'benchmark-api-ws'

// A catalog entry with no fetch handler is a dead end: `call_api_get` builds a
// relative `/api/...` url, the stub declines it, and node's fetch throws on the
// relative url instead of returning a result the model can act on. The model then
// burns its remaining turns searching for an endpoint that will never answer.
describe('benchmark API catalog', () => {
	beforeEach(() => resetBenchmarkMockBackend())
	afterEach(() => resetBenchmarkMockBackend())

	it('answers every GET endpoint it advertises', () => {
		const unanswered = listBenchmarkMcpTools()
			.filter((tool) => tool.method.toUpperCase() === 'GET')
			.map((tool) =>
				`/api${tool.path.replace('{workspace}', WORKSPACE)}`.replace(/\{[^}]+\}/g, 'x')
			)
			.filter((url) => !hasBenchmarkApiHandler(url))

		// The draft-covered reads are refused by name before any fetch, so they are
		// advertised without a handler on purpose.
		expect(unanswered).toEqual([
			`/api/w/${WORKSPACE}/scripts/get/p/x`,
			`/api/w/${WORKSPACE}/variables/get/x`
		])
	})

	it('serves a recorded job so a model can check the run it just started', async () => {
		const id = createBenchmarkCompletedJob({
			workspace: WORKSPACE,
			jobKind: 'preview',
			result: 'Hello, World!'
		})

		const res = handleBenchmarkApiFetch(`/api/w/${WORKSPACE}/jobs_u/get/${id}`)

		expect(res.status).toBe(200)
		expect(await res.json()).toMatchObject({
			id,
			success: true,
			result: 'Hello, World!'
		})
	})

	it('404s an unknown job id instead of letting the fetch fall through', () => {
		expect(hasBenchmarkApiHandler(`/api/w/${WORKSPACE}/jobs_u/get/missing`)).toBe(true)
		expect(handleBenchmarkApiFetch(`/api/w/${WORKSPACE}/jobs_u/get/missing`).status).toBe(404)
	})
})
