import { beforeEach, describe, expect, it, vi } from 'vitest'

const { getMcpToolsMock, callMcpToolMock } = vi.hoisted(() => ({
	getMcpToolsMock: vi.fn(),
	callMcpToolMock: vi.fn()
}))

vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

vi.mock('$lib/gen', () => ({
	ResourceService: {
		getMcpTools: getMcpToolsMock,
		callMcpTool: callMcpToolMock
	}
}))

import { clearMcpToolsCache, createMcpTools, type McpServer } from './mcpTools'

const SERVERS: McpServer[] = [{ path: 'u/hugo/github_mcp' }]

const TOOLS = [
	{
		name: 'get_issue',
		description: 'Get details of a GitHub issue',
		inputSchema: {
			type: 'object',
			properties: { owner: { type: 'string' }, repo: { type: 'string' } },
			required: ['owner', 'repo']
		},
		annotations: { readOnlyHint: true }
	},
	{
		name: 'merge_pull_request',
		description: 'Merge a pull request',
		inputSchema: { type: 'object', properties: { pull_number: { type: 'number' } } },
		annotations: { readOnlyHint: false }
	},
	{
		// No annotations at all: must be treated as mutating, never as read-only.
		name: 'unannotated_tool',
		description: 'A tool the server tells us nothing about',
		inputSchema: { type: 'object', properties: {} }
	}
]

function createToolCallbacks() {
	return {
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	} as any
}

function getTool(name: string) {
	const tool = createMcpTools(SERVERS).find((entry) => entry.def.function.name === name)
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
	clearMcpToolsCache()
	getMcpToolsMock.mockResolvedValue(TOOLS)
	callMcpToolMock.mockReset()
})

describe('tool registration', () => {
	it('registers nothing when no MCP server is connected', () => {
		expect(createMcpTools([])).toEqual([])
	})
})

describe('read/write split', () => {
	it('refuses a mutating tool on the read path', async () => {
		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'merge_pull_request'
		})
		expect(result.success).toBe(false)
		expect(result.error).toContain('call_mcp_write_tool')
	})

	it('refuses an unannotated tool on the read path', async () => {
		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'unannotated_tool'
		})
		expect(result.success).toBe(false)
		expect(result.error).toContain('call_mcp_write_tool')
	})

	it('refuses a read-only tool on the write path', async () => {
		const result = await run('call_mcp_write_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue'
		})
		expect(result.success).toBe(false)
		expect(result.error).toContain('call_mcp_read_tool')
	})

	it('asks for confirmation only on the write path', () => {
		expect(getTool('call_mcp_read_tool').requiresConfirmation).toBeFalsy()
		expect(getTool('call_mcp_write_tool').requiresConfirmation).toBe(true)
	})
})

describe('call results', () => {
	it('returns the tool argument schema when the call is rejected', async () => {
		callMcpToolMock.mockRejectedValue({
			status: 400,
			body: { error: { message: 'missing owner' } }
		})
		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue',
			arguments: {}
		})
		expect(result.success).toBe(false)
		expect(result.schema).toEqual(TOOLS[0].inputSchema)
	})

	it('reports a tool that ran but returned isError as a failure', async () => {
		callMcpToolMock.mockResolvedValue({
			content: [{ type: 'text', text: 'issue not found' }],
			isError: true
		})
		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue',
			arguments: { owner: 'a', repo: 'b' }
		})
		expect(result.success).toBe(false)
		expect(result.error).toBe('issue not found')
	})

	it('flattens text content on success', async () => {
		callMcpToolMock.mockResolvedValue({ content: [{ type: 'text', text: '{"number":42}' }] })
		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue',
			arguments: { owner: 'a', repo: 'b' }
		})
		expect(result).toEqual({ success: true, data: '{"number":42}' })
	})
})

describe('search_mcp_tools', () => {
	it('returns compact summaries without the full input schemas', async () => {
		const result = await run('search_mcp_tools', { query: 'issue' })
		expect(result.matches).toEqual([
			{
				server: 'u/hugo/github_mcp',
				tool: 'get_issue',
				description: 'Get details of a GitHub issue',
				mode: 'read',
				params: ['owner', 'repo']
			}
		])
	})

	it('still returns matches when one server is unreachable', async () => {
		const servers: McpServer[] = [...SERVERS, { path: 'u/hugo/broken_mcp' }]
		getMcpToolsMock.mockImplementation(({ path }: { path: string }) =>
			path === 'u/hugo/broken_mcp'
				? Promise.reject(new Error('connection refused'))
				: Promise.resolve(TOOLS)
		)
		const tool = createMcpTools(servers).find(
			(entry) => entry.def.function.name === 'search_mcp_tools'
		)!
		const result = JSON.parse(
			await tool.fn({
				args: { query: 'issue' },
				workspace: 'test-ws',
				helpers: {},
				toolCallbacks: createToolCallbacks(),
				toolId: 'tool-1'
			})
		)
		expect(result.matches).toHaveLength(1)
		expect(result.unavailable).toHaveLength(1)
		expect(result.unavailable[0]).toContain('u/hugo/broken_mcp')
	})
})
