import { beforeEach, describe, expect, it, vi } from 'vitest'

const { getMcpToolsMock, callMcpToolMock, listResourceMock, session } = vi.hoisted(() => ({
	getMcpToolsMock: vi.fn(),
	callMcpToolMock: vi.fn(),
	listResourceMock: vi.fn(),
	session: { email: 'first@windmill.dev' }
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
		callMcpTool: callMcpToolMock,
		listResource: listResourceMock
	}
}))

vi.mock('$lib/stores', () => ({
	// Read at call time, so a test can switch accounts the way a logout does.
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ ...session }), () => {}) }
}))

import { clearMcpToolsCache, createMcpTools, loadMcpServers, type McpServer } from './mcpTools'
import { setMcpEnabled } from '$lib/components/mcp/enabledServers'

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

// A server controls its error text as much as its output, so the cap has to hold
// on the failure path too.
describe('result size cap', () => {
	it('truncates an oversized isError payload', async () => {
		getMcpToolsMock.mockResolvedValue(TOOLS)
		callMcpToolMock.mockResolvedValue({
			isError: true,
			content: [{ type: 'text', text: 'x'.repeat(80_000) }]
		})
		const result = JSON.parse(
			await getTool('call_mcp_read_tool').fn({
				args: { server: 'u/hugo/github_mcp', tool: 'get_issue', arguments: {} },
				workspace: 'test-ws',
				helpers: {},
				toolCallbacks: createToolCallbacks(),
				toolId: 'tool-1'
			})
		)
		expect(result.success).toBe(false)
		expect(result.truncated).toBe(true)
		expect(result.error.length).toBeLessThanOrEqual(20_000)
	})
})

// The opt-in boundary: a readable `mcp` resource is not a server the chat may
// act through until its owner turns it on.
describe('loadMcpServers', () => {
	beforeEach(() => {
		localStorage.clear()
		listResourceMock.mockResolvedValue([
			{ path: 'u/hugo/github_mcp' },
			{ path: 'f/team/shared_mcp' }
		])
	})

	it('advertises nothing while no server is enabled, without listing resources', async () => {
		expect(await loadMcpServers('test-ws')).toEqual([])
		expect(listResourceMock).not.toHaveBeenCalled()
	})

	it('advertises only the enabled server', async () => {
		setMcpEnabled('test-ws', 'u/hugo/github_mcp', true)
		expect(await loadMcpServers('test-ws')).toEqual([{ path: 'u/hugo/github_mcp' }])
	})

	it('does not carry an enabled server into another workspace', async () => {
		setMcpEnabled('test-ws', 'u/hugo/github_mcp', true)
		expect(await loadMcpServers('other-ws')).toEqual([])
	})

	// Browser storage outlives a logout, so the next account must not inherit
	// tools the previous one turned on.
	it('does not carry an enabled server across accounts in the same browser', async () => {
		setMcpEnabled('test-ws', 'f/team/shared_mcp', true)
		session.email = 'second@windmill.dev'
		try {
			expect(await loadMcpServers('test-ws')).toEqual([])
		} finally {
			session.email = 'first@windmill.dev'
		}
	})
})
