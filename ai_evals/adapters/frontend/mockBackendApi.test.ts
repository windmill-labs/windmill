import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import {
	createBenchmarkCompletedJob,
	getBenchmarkCompletedJob,
	handleBenchmarkApiFetch,
	hasBenchmarkApiHandler,
	resetBenchmarkMockBackend,
	registerBenchmarkWorkspaceRunnables
} from './mockBackend'

const WORKSPACE = 'benchmark-api-ws'

// A relative `/api/...` call has no server to reach in this environment: without a
// handler here the stub declines it and node's fetch throws, instead of returning a
// result the model can act on.
describe('benchmark API fetch handlers', () => {
	beforeEach(() => resetBenchmarkMockBackend())
	afterEach(() => resetBenchmarkMockBackend())

	it('runs a deployed script by path', async () => {
		registerBenchmarkWorkspaceRunnables(WORKSPACE, {
			scripts: [
				{
					path: 'f/evals/greet',
					summary: 'Greet',
					language: 'bun',
					content: 'export async function main() {}'
				}
			]
		})

		const res = handleBenchmarkApiFetch(
			`/api/w/${WORKSPACE}/jobs/run/p/${encodeURIComponent('f/evals/greet')}`,
			{ method: 'POST', body: JSON.stringify({ name: 'ada' }) }
		)

		expect(res.status).toBe(200)
		const job = getBenchmarkCompletedJob(WORKSPACE, (await res.text()).trim())
		expect(job).toMatchObject({ success: true, args: { name: 'ada' } })
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
