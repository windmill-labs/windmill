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
	},
	{
		name: 'getVariable',
		description: 'Get variable',
		instructions: '',
		path: '/w/{workspace}/variables/get/{path}',
		method: 'GET'
	},
	{
		name: 'deleteScriptByHash',
		description: 'Delete a script by hash',
		instructions: '',
		path: '/w/{workspace}/scripts/delete/h/{hash}',
		method: 'POST',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, hash: { type: 'string' } },
			required: ['workspace', 'hash']
		}
	},
	{
		name: 'runFlowByPath',
		description: 'Run flow by path',
		instructions: 'Trigger a run of a deployed flow',
		path: '/w/{workspace}/jobs/run/f/{path}',
		method: 'POST',
		path_params_schema: {
			type: 'object',
			properties: { workspace: { type: 'string' }, path: { type: 'string' } },
			required: ['workspace', 'path']
		},
		body_schema: {
			type: 'object',
			properties: { args: { type: 'object' } }
		}
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
		expect(result.matches[0].params).toEqual(['page', 'per_page'])
		expect(result.matches[0].instructions).toContain('ping status')

		const flows = await run('search_api_endpoints', { query: 'create flow' })
		expect(flows.matches.map((m: any) => m.name)).not.toContain('createFlow')
		expect(flows.covered_by_dedicated_tools).toContain('createFlow → use write_flow')
	})

	it('returns endpoint categories when nothing matches', async () => {
		const result = await run('search_api_endpoints', { query: 'kubernetes' })
		expect(result.matches).toEqual([])
		expect(result.hint).toContain('workers')
		expect(result.hint).toContain('jobs')
	})
})

describe('call_api_get', () => {
	it('rejects covered, unknown, and non-GET endpoints with a pointer', async () => {
		const covered = await run('call_api_get', { name: 'createFlow' })
		expect(covered.error).toContain('write_flow')

		const unknown = await run('call_api_get', { name: 'nope' })
		expect(unknown.error).toContain('search_api_endpoints')

		const mutating = await run('call_api_get', { name: 'runFlowByPath' })
		expect(mutating.error).toContain('call_api_endpoint')

		const deleting = await run('call_api_endpoint', { name: 'deleteSchedule' })
		expect(deleting.error).toContain('delete_workspace_item')

		const byHash = await run('call_api_endpoint', { name: 'deleteScriptByHash' })
		expect(byHash.error).toContain('delete_workspace_item')
		const search = await run('search_api_endpoints', { query: 'delete script' })
		expect(search.matches.map((m: any) => m.name)).not.toContain('deleteScriptByHash')
	})

	it('refuses variable reads so variable values never reach the model', async () => {
		const result = await run('call_api_get', { name: 'getVariable' })
		expect(result.success).toBe(false)
		expect(result.error).toContain('never readable')

		const search = await run('search_api_endpoints', { query: 'variable' })
		expect(search.matches.map((m: any) => m.name)).not.toContain('getVariable')
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
	it('requires confirmation and executes mutating endpoints with a body', async () => {
		expect(getTool('call_api_endpoint').requiresConfirmation).toBe(true)
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			headers: new Headers({ 'content-type': 'text/plain' }),
			text: async () => 'job-id-1'
		})
		vi.stubGlobal('fetch', fetchMock)
		const result = await run('call_api_endpoint', {
			name: 'runFlowByPath',
			params: { path: 'u/me/myflow' },
			body: { args: { n: 1 } }
		})
		expect(fetchMock).toHaveBeenCalledWith('/api/w/test-ws/jobs/run/f/u%2Fme%2Fmyflow', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ args: { n: 1 } })
		})
		expect(result).toEqual({ success: true, data: 'job-id-1' })
	})

	it('redirects GET endpoints to call_api_get', async () => {
		const result = await run('call_api_endpoint', { name: 'listWorkers' })
		expect(result.error).toContain('call_api_get')
	})
})
