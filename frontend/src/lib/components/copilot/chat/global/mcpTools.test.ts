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

async function run(name: string, args: Record<string, unknown>, workspace = 'test-ws') {
	const raw = await getTool(name).fn({
		args,
		workspace,
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

	// The rejection sends the model to the write tool, which classifies from the
	// same cached listing: without dropping it, that retry is refused too and the
	// model has nowhere to go until the entry expires.
	it('reclassifies after the backend refuses the read-only assertion', async () => {
		callMcpToolMock.mockRejectedValueOnce({
			status: 400,
			body: 'Bad request: MCP tool get_issue is not marked read-only by the server, it must be called as a tool that modifies data'
		})
		const refused = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue'
		})
		expect(refused.success).toBe(false)

		// The server now reports the same name as mutating.
		getMcpToolsMock.mockResolvedValue([{ ...TOOLS[0], annotations: { readOnlyHint: false } }])
		callMcpToolMock.mockResolvedValue({ content: [] })
		const retried = await run('call_mcp_write_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue'
		})
		expect(retried.success).toBe(true)
	})

	// This classification comes from a listing that can predate a resource edited
	// mid-turn, so the backend re-checks it against the server it is calling — but
	// only knows to when the unconfirmed path says it assumed read-only.
	it('tells the backend when it called without a confirmation', async () => {
		callMcpToolMock.mockResolvedValue({ content: [] })
		await run('call_mcp_read_tool', { server: 'u/hugo/github_mcp', tool: 'get_issue' })
		expect(callMcpToolMock.mock.calls[0][0].requestBody.read_only).toBe(true)

		await run('call_mcp_write_tool', { server: 'u/hugo/github_mcp', tool: 'merge_pull_request' })
		expect(callMcpToolMock.mock.calls[1][0].requestBody.read_only).toBeUndefined()
	})

	// The cached tool list carries the annotations this gate reads, and the same
	// path names a different server in another workspace: a cache keyed on path
	// alone would let one workspace's read-only hint wave a call through in the next.
	it("does not reuse one workspace's tool list in another", async () => {
		callMcpToolMock.mockResolvedValue({ content: [] })
		await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue',
			arguments: {}
		})
		getMcpToolsMock.mockResolvedValue([{ ...TOOLS[0], annotations: { readOnlyHint: false } }])

		const result = await run(
			'call_mcp_read_tool',
			{ server: 'u/hugo/github_mcp', tool: 'get_issue', arguments: {} },
			'other-ws'
		)

		expect(getMcpToolsMock).toHaveBeenCalledTimes(2)
		expect(result.success).toBe(false)
		expect(result.error).toContain('call_mcp_write_tool')
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
	// A listing in flight when a path is reconnected must not land in the cache it
	// was cleared from: it would answer for the new server with the old server's
	// annotations, and those decide whether a call needs confirmation.
	it('drops a tool list that was already in flight when the cache was cleared', async () => {
		let release: (tools: unknown) => void = () => {}
		getMcpToolsMock.mockReturnValueOnce(new Promise((resolve) => (release = resolve)))
		const inFlight = run('search_mcp_tools', { query: 'issue' })

		clearMcpToolsCache()
		release(TOOLS)
		await inFlight

		getMcpToolsMock.mockResolvedValue(TOOLS)
		await run('search_mcp_tools', { query: 'issue' })
		expect(getMcpToolsMock).toHaveBeenCalledTimes(2)
	})

	// A path can be reconnected to a different server through the resource UI, which
	// this module never hears about. The listing carries the annotations the gate
	// reads, so it is keyed on the revision rather than on the path alone.
	it('does not reuse a tool list across a resource revision', async () => {
		callMcpToolMock.mockResolvedValue({ content: [] })
		const call = (editedAt: string) =>
			createMcpTools([{ path: 'u/hugo/github_mcp', editedAt }])
				.find((t) => t.def.function.name === 'call_mcp_read_tool')!
				.fn({
					args: { server: 'u/hugo/github_mcp', tool: 'get_issue', arguments: {} },
					workspace: 'test-ws',
					helpers: {},
					toolCallbacks: createToolCallbacks(),
					toolId: 'tool-1'
				})

		await call('2026-01-01T00:00:00Z')
		getMcpToolsMock.mockResolvedValue([{ ...TOOLS[0], annotations: { readOnlyHint: false } }])
		const result = JSON.parse(await call('2026-01-02T00:00:00Z'))

		expect(getMcpToolsMock).toHaveBeenCalledTimes(2)
		expect(result.success).toBe(false)
		expect(result.error).toContain('call_mcp_write_tool')
	})

	it('refuses to answer a call from a listing that was invalidated mid-flight', async () => {
		// Invalidated while in flight, every time: the tool list may describe the
		// server that was replaced, and its `readOnlyHint` is what decides whether
		// the call needs confirmation.
		getMcpToolsMock.mockImplementation(async () => {
			clearMcpToolsCache()
			return TOOLS
		})

		const result = await run('call_mcp_read_tool', {
			server: 'u/hugo/github_mcp',
			tool: 'get_issue',
			arguments: {}
		})

		expect(result.success).toBe(false)
		expect(callMcpToolMock).not.toHaveBeenCalled()
	})

	it('truncates an oversized tools/list failure in search', async () => {
		getMcpToolsMock.mockRejectedValue(new Error('x'.repeat(80_000)))
		const result = await run('search_mcp_tools', { query: 'issue' })
		expect(result.unavailable[0].length).toBeLessThan(1_000)
	})

	// Escaping is the server's to control: a run of backslashes doubles under
	// JSON.stringify, so a cap measured before serializing is not a cap.
	it('holds the cap on escape-heavy output', async () => {
		callMcpToolMock.mockResolvedValue({
			content: [{ type: 'text', text: '\\'.repeat(60_000) }]
		})
		const raw = await getTool('call_mcp_read_tool').fn({
			args: { server: 'u/hugo/github_mcp', tool: 'get_issue', arguments: {} },
			workspace: 'test-ws',
			helpers: {},
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-1'
		})
		expect(raw.length).toBeLessThanOrEqual(20_000)
	})

	it('truncates an oversized isError payload', async () => {
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
