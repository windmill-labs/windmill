import { beforeEach, describe, expect, it, vi } from 'vitest'

const { listMcpToolsMock } = vi.hoisted(() => ({
	listMcpToolsMock: vi.fn()
}))

vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

vi.mock('$lib/gen', () => ({
	McpService: {
		listMcpTools: listMcpToolsMock
	}
}))

import { apiCatalogTools, clearApiCatalogCache } from './apiCatalogTools'

const CATALOG = [
	{
		name: 'listWorkers',
		description: 'List workers',
		instructions: 'List all workers with their ping status',
		path: '/workers/list',
		method: 'GET',
		query_params_schema: {
			type: 'object',
			properties: { page: { type: 'integer' }, per_page: { type: 'integer' } }
		}
	},
	{
		name: 'getJob',
		description: 'Get job details',
		instructions: '',
		path: '/w/{workspace}/jobs_u/get/{id}',
		method: 'GET',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, id: { type: 'string' } },
			required: ['workspace', 'id']
		}
	},
	{
		name: 'deleteSchedule',
		description: 'Delete a schedule',
		instructions: '',
		path: '/w/{workspace}/schedules/delete/{path}',
		method: 'DELETE',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, path: { type: 'string' } },
			required: ['workspace', 'path']
		}
	},
	{
		name: 'createFlow',
		description: 'Create a flow',
		instructions: '',
		path: '/w/{workspace}/flows/create',
		method: 'POST'
	}
]

function createToolCallbacks() {
	return {
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	} as any
}

function getTool(name: string) {
	const tool = apiCatalogTools.find((entry) => entry.def.function.name === name)
	if (!tool) throw new Error(`${name} tool not found`)
	return tool
}

async function run(name: string, args: Record<string, unknown>) {
	const raw = await getTool(name).fn({
		args,
		workspace: 'test-ws',
		helpers: {},
		toolCallbacks: createToolCallbacks(),
		toolId: 'tool-1'
	})
	return JSON.parse(raw)
}

beforeEach(() => {
	vi.clearAllMocks()
	clearApiCatalogCache()
	listMcpToolsMock.mockResolvedValue(CATALOG)
	vi.unstubAllGlobals()
})

describe('search_api_endpoints', () => {
	it('matches on name/path tokens, plural-insensitively, and excludes covered endpoints', async () => {
		const result = await run('search_api_endpoints', { query: 'worker' })
		expect(result.matches.map((m: any) => m.name)).toEqual(['listWorkers'])
		expect(result.matches[0].endpoint).toBe('GET /workers/list')

		const flows = await run('search_api_endpoints', { query: 'create flow' })
		expect(flows.matches.map((m: any) => m.name)).not.toContain('createFlow')
		expect(flows.covered_by_dedicated_tools).toEqual(['createFlow → use write_flow'])
	})

	it('returns endpoint categories when nothing matches', async () => {
		const result = await run('search_api_endpoints', { query: 'kubernetes' })
		expect(result.matches).toEqual([])
		expect(result.hint).toContain('workers')
		expect(result.hint).toContain('schedules')
	})
})

describe('call_api_get', () => {
	it('rejects covered, unknown, and non-GET endpoints with a pointer', async () => {
		const covered = await run('call_api_get', { name: 'createFlow' })
		expect(covered.error).toContain('write_flow')

		const unknown = await run('call_api_get', { name: 'nope' })
		expect(unknown.error).toContain('search_api_endpoints')

		const mutating = await run('call_api_get', { name: 'deleteSchedule' })
		expect(mutating.error).toContain('call_api_endpoint')
	})

	it('returns the endpoint schema when a required path param is missing', async () => {
		const result = await run('call_api_get', { name: 'getJob' })
		expect(result.success).toBe(false)
		expect(result.error).toContain('id')
		expect(result.schema.path_params_schema.required).toContain('id')
	})

	it('substitutes path params and sends the rest as query params', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'application/json' }),
			json: async () => [{ worker: 'w1' }]
		})
		vi.stubGlobal('fetch', fetchMock)
		const result = await run('call_api_get', { name: 'listWorkers', params: { page: 2 } })
		expect(fetchMock).toHaveBeenCalledWith('/api/workers/list?page=2', { method: 'GET' })
		expect(result).toEqual({ success: true, data: [{ worker: 'w1' }] })
	})
})

describe('call_api_endpoint', () => {
	it('requires confirmation and executes mutating endpoints', async () => {
		expect(getTool('call_api_endpoint').requiresConfirmation).toBe(true)
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'text/plain' }),
			text: async () => 'deleted'
		})
		vi.stubGlobal('fetch', fetchMock)
		const result = await run('call_api_endpoint', {
			name: 'deleteSchedule',
			params: { path: 'u/me/sched' }
		})
		expect(fetchMock).toHaveBeenCalledWith('/api/w/test-ws/schedules/delete/u%2Fme%2Fsched', {
			method: 'DELETE'
		})
		expect(result).toEqual({ success: true, data: 'deleted' })
	})

	it('redirects GET endpoints to call_api_get', async () => {
		const result = await run('call_api_endpoint', { name: 'listWorkers' })
		expect(result.error).toContain('call_api_get')
	})
})
