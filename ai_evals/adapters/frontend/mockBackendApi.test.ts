import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import {
	createBenchmarkCompletedJob,
	getBenchmarkCompletedJob,
	handleBenchmarkApiFetch,
	hasBenchmarkApiHandler,
	listBenchmarkMcpTools,
	resetBenchmarkMockBackend,
	registerBenchmarkWorkspaceRunnables
} from './mockBackend'

const WORKSPACE = 'benchmark-api-ws'

// A catalog entry with no fetch handler is a dead end: the catalog executor builds a
// relative `/api/...` url, the stub declines it, and node's fetch throws on the relative
// url instead of returning a result the model can act on. Mutating entries are reachable
// too — the eval runners define no `requestConfirmation`, so `call_api_endpoint` executes
// unconfirmed.
describe('benchmark API catalog', () => {
	beforeEach(() => resetBenchmarkMockBackend())
	afterEach(() => resetBenchmarkMockBackend())

	it('answers every endpoint it advertises', () => {
		const unanswered = listBenchmarkMcpTools()
			.map((tool) =>
				`/api${tool.path.replace('{workspace}', WORKSPACE)}`.replace(/\{[^}]+\}/g, 'x')
			)
			.filter((url) => !hasBenchmarkApiHandler(url))

		// The draft-covered entries are refused by name before any fetch, so they are
		// advertised without a handler on purpose.
		expect(unanswered).toEqual([
			`/api/w/${WORKSPACE}/scripts/get/p/x`,
			`/api/w/${WORKSPACE}/flows/create`,
			`/api/w/${WORKSPACE}/schedules/delete/x`,
			`/api/w/${WORKSPACE}/variables/get/x`
		])
	})

	it('runs a deployed script by path, the way call_api_endpoint reaches it', async () => {
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

// A benchmark whose list calls an integration undocumented while its metadata endpoint
// hands back authored notes teaches the model the flag means nothing.
describe('benchmark hub integration list', () => {
	beforeEach(() => resetBenchmarkMockBackend())

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
