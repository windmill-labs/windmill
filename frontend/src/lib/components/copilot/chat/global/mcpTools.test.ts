import { beforeEach, describe, expect, it, vi } from 'vitest'

const { getMcpToolsMock, callMcpToolMock, listResourceMock, session } = vi.hoisted(() => ({
	getMcpToolsMock: vi.fn(),
	callMcpToolMock: vi.fn(),
	listResourceMock: vi.fn(),
	session: { email: 'first@windmill.dev' }
}))

// `../shared` is stubbed because it reaches the chat's stores. The schema normalizer
// that sanitizes a remote `inputSchema` on its way into a provider request is
// deliberately NOT here — it lives in `../toolSchema`, a leaf module with no such
// imports, so these tests exercise the real one rather than a copy that could drift.
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

import {
	clearMcpToolsCache,
	createMcpTools,
	forgetLoadedMcpTools,
	invalidateMcpRegistrations,
	isRequestBodyRejection,
	loadedMcpServers,
	mcpRegistryGeneration,
	loadedMcpTools,
	loadMcpServers,
	registerMcpTools,
	type McpServer
} from './mcpTools'
import { setMcpEnabled } from '$lib/components/mcp/enabledServers'

const SERVERS: McpServer[] = [{ path: 'u/hugo/github_mcp' }]
// Each chat manager scopes its own registry; these tests act as one of them.
const OWNER = 'owner-a'

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
	const tool = createMcpTools(OWNER, SERVERS).find((entry) => entry.def.function.name === name)
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
		expect(createMcpTools(OWNER, [])).toEqual([])
	})
})

describe('loaded remote tools', () => {
	const server = SERVERS[0]

	it('registers a remote tool under its own schema and server-scoped name', () => {
		const [name] = registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0]])

		expect(name).toBe('mcp_u_hugo_github_mcp__get_issue')
		const tool = loadedMcpTools(OWNER).find((t) => t.def.function.name === name)
		expect(tool?.def.function.parameters).toEqual(TOOLS[0].inputSchema)
	})

	// The wrappers ask for confirmation by which one the model picked; a registered
	// tool carries its own hint, so the gate is per tool.
	it('gates a mutating tool and lets a read-only one through', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0], TOOLS[1], TOOLS[2]])
		const byName = Object.fromEntries(loadedMcpTools(OWNER).map((t) => [t.def.function.name, t]))

		expect(byName['mcp_u_hugo_github_mcp__get_issue'].requiresConfirmation).toBeUndefined()
		expect(byName['mcp_u_hugo_github_mcp__merge_pull_request'].requiresConfirmation).toBe(true)
		// No annotations at all must not read as read-only.
		expect(byName['mcp_u_hugo_github_mcp__unannotated_tool'].requiresConfirmation).toBe(true)
	})

	// A registered tool freezes a copy of `readOnlyHint`, which is what the listing
	// TTL exists to bound — so it must not outlive the listing it came from.
	it('drops loaded tools when the listing cache is cleared', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0]])
		expect(loadedMcpTools(OWNER)).toHaveLength(1)

		clearMcpToolsCache()

		expect(loadedMcpTools(OWNER)).toEqual([])
	})

	it('drops only the named server when reconciling against the live list', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0]])
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), { path: 'f/team/linear_mcp' }, [TOOLS[0]])
		expect(
			loadedMcpServers(OWNER)
				.map((s) => s.path)
				.sort()
		).toEqual(['f/team/linear_mcp', 'u/hugo/github_mcp'])

		forgetLoadedMcpTools(OWNER, 'f/team/linear_mcp')

		expect(loadedMcpServers(OWNER).map((s) => s.path)).toEqual(['u/hugo/github_mcp'])
	})

	// The reported bug: the model saw only parameter names, so it invented values for
	// constrained arguments (Linear's `orderBy`, whose enum it never saw). Searching
	// must put the real schema in front of it, not a list of names.
	it('registers what a search matched, with the remote schema', async () => {
		setMcpEnabled('test-ws', 'u/hugo/github_mcp', true)
		const result = await run('search_mcp_tools', { query: 'issue' })

		const match = result.matches.find((m: any) => m.tool === 'get_issue')
		expect(match.call).toBe('mcp_u_hugo_github_mcp__get_issue')
		expect(match.params).toBeUndefined()

		const registered = loadedMcpTools(OWNER).find((t) => t.def.function.name === match.call)
		expect(registered?.def.function.parameters).toEqual(TOOLS[0].inputSchema)
	})

	// The row's provider mark is resolved from this alone — the registry lives only in
	// memory and the live server list is empty on a reloaded session — so it has to land
	// on the message, and before the listing, which an unreachable server never answers.
	it('records the server on the row, even when it cannot be reached', async () => {
		getMcpToolsMock.mockRejectedValue(new Error('connection refused'))
		const callbacks = createToolCallbacks()

		await getTool('call_mcp_read_tool').fn({
			args: { server: 'u/hugo/github_mcp', tool: 'get_issue', arguments: {} },
			workspace: 'test-ws',
			helpers: {},
			toolCallbacks: callbacks,
			toolId: 'tool-1'
		})

		expect(callbacks.setToolStatus).toHaveBeenCalledWith('tool-1', {
			mcpServer: 'u/hugo/github_mcp'
		})
	})

	// The transcript trusts this field without re-checking it, so it must come from the
	// connected list rather than from the arguments — otherwise a path the model made up
	// would be resolved, and resolving one reads that workspace resource.
	it('records nothing for a server the user has not connected', async () => {
		const callbacks = createToolCallbacks()

		await getTool('call_mcp_read_tool').fn({
			args: { server: 'u/hugo/not_connected', tool: 'get_issue', arguments: {} },
			workspace: 'test-ws',
			helpers: {},
			toolCallbacks: callbacks,
			toolId: 'tool-1'
		})

		const recorded = callbacks.setToolStatus.mock.calls.map(([, meta]: [string, any]) => meta)
		expect(recorded.some((m: any) => m?.mcpServer !== undefined)).toBe(false)
	})

	// Several chats are live at once — the docked one plus a warm runtime per session,
	// all in GLOBAL mode. A shared registry would put one chat's remote tools into
	// another's request, and let either one's "New chat" drop the other's.
	it('scopes registered tools to their owner', () => {
		const other = 'owner-b'
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0]])
		registerMcpTools(other, mcpRegistryGeneration(other), server, [TOOLS[1]])

		expect(loadedMcpTools(OWNER).map((t) => t.def.function.name)).toEqual([
			'mcp_u_hugo_github_mcp__get_issue'
		])
		expect(loadedMcpTools(other).map((t) => t.def.function.name)).toEqual([
			'mcp_u_hugo_github_mcp__merge_pull_request'
		])

		forgetLoadedMcpTools(OWNER)

		expect(loadedMcpTools(OWNER)).toEqual([])
		expect(loadedMcpTools(other)).toHaveLength(1)
	})

	// A registered tool's schema goes into the request, where a provider rejects the
	// whole completion over one bad tool — and the tool stays registered, so every
	// later send fails too. These are shapes Windmill's own MCP server has emitted.
	it('makes a request-breaking remote schema safe before registering it', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
			{
				name: 'broken',
				description: 'x',
				inputSchema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					type: 'string',
					// `format: ''` is what Windmill's own MCP server emits for an untyped
					// field, and only the recursive normalizer removes it.
					properties: { a: { type: 'string', format: '' } },
					required: ['a', 'a', 'ghost']
				}
			} as any
		])

		const params = loadedMcpTools(OWNER)[0].def.function.parameters as any
		expect(params.type).toBe('object')
		expect(params.required).toEqual(['a'])
		expect(params.$schema).toBeUndefined()
		expect(params.properties.a.format).toBeUndefined()
	})

	// A registered schema rides in every request for the rest of the conversation, so
	// an outsized one is left to the wrapper — where it costs a tool result once —
	// rather than truncated into something the model cannot tell is incomplete.
	it('does not register a tool whose schema is too large to carry', () => {
		const huge = {
			name: 'huge',
			description: 'x',
			inputSchema: {
				type: 'object',
				properties: { a: { type: 'string', description: 'x'.repeat(9000) } }
			}
		} as any

		expect(registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [huge])).toEqual([
			undefined
		])
		expect(loadedMcpTools(OWNER)).toEqual([])
	})

	// The sanitized schema must not alias the listing cache: the normalize pass rewrites
	// nested nodes in place, so a shallow copy would edit the entry `summarizeTool` and
	// the failed-call schema echo read from, and that every other chat shares.
	it('does not mutate the remote schema it was given', () => {
		const remote = {
			name: 'aliased',
			description: 'x',
			inputSchema: {
				type: 'object',
				properties: { a: { type: 'string', format: '' } },
				required: ['a', 'a']
			}
		} as any

		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [remote])

		expect(remote.inputSchema.properties.a.format).toBe('')
		expect(remote.inputSchema.required).toEqual(['a', 'a'])
	})

	it('falls back to an empty object schema when the remote sends no usable one', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
			{ name: 'nada', description: 'x', inputSchema: null } as any
		])

		expect(loadedMcpTools(OWNER)[0].def.function.parameters).toEqual({
			type: 'object',
			properties: {}
		})
	})

	// The emitted list carries Anthropic's cache_control breakpoint on its last entry,
	// so calling or re-registering a tool must not move it: a reorder re-processes the
	// whole cached prefix on the next iteration.
	it('keeps the emitted tool order stable across calls and re-registration', async () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0], TOOLS[1]])
		const before = loadedMcpTools(OWNER).map((t) => t.def.function.name)

		// Call the first-registered one, then re-register the pair.
		await loadedMcpTools(OWNER)[0].fn({
			args: {},
			workspace: 'test-ws',
			helpers: {},
			toolCallbacks: createToolCallbacks(),
			toolId: 'tool-1'
		})
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [TOOLS[0], TOOLS[1]])

		expect(loadedMcpTools(OWNER).map((t) => t.def.function.name)).toEqual(before)
	})

	// The per-tool bound alone would allow 25 large schemas in every request, which is
	// the cost the whole search-then-register indirection exists to avoid.
	it('bounds the total size of the registered schemas', () => {
		const big = (name: string) => ({
			name,
			description: 'x',
			inputSchema: {
				type: 'object',
				properties: { a: { type: 'string', description: 'x'.repeat(7_000) } }
			}
		})
		for (let i = 0; i < 12; i++) {
			registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [big(`tool_${i}`)] as any)
		}

		const total = loadedMcpTools(OWNER).reduce(
			(n, t) => n + JSON.stringify(t.def.function.parameters).length,
			0
		)
		expect(total).toBeLessThanOrEqual(40_000)
		expect(loadedMcpTools(OWNER).length).toBeLessThan(12)
	})

	it('bounds the loaded set, evicting the least recently registered', () => {
		for (let i = 0; i < 30; i++) {
			registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
				{ ...TOOLS[0], name: `tool_${i}` }
			])
		}
		const names = loadedMcpTools(OWNER).map((t) => t.def.function.name)

		expect(names).toHaveLength(25)
		expect(names).not.toContain('mcp_u_hugo_github_mcp__tool_0')
		expect(names).toContain('mcp_u_hugo_github_mcp__tool_29')
	})

	// Registering has to count as a use. Ranking a fresh registration below every
	// already-called tool evicted it the moment it arrived, while the search that
	// asked for it had already advertised its name — unrecoverable by searching again.
	it('keeps a newly searched tool when every registered tool has been called', async () => {
		for (let i = 0; i < 25; i++) {
			registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
				{ ...TOOLS[0], name: `tool_${i}` }
			])
		}
		for (const tool of loadedMcpTools(OWNER)) {
			await tool.fn({
				args: {},
				workspace: 'test-ws',
				helpers: {},
				toolCallbacks: createToolCallbacks(),
				toolId: 'tool-1'
			})
		}

		const [name] = registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
			{ ...TOOLS[0], name: 'fresh' }
		])

		expect(name).toBe('mcp_u_hugo_github_mcp__fresh')
		expect(loadedMcpTools(OWNER).map((t) => t.def.function.name)).toContain(name)
	})

	// Sanitizing maps both `/` and `_` to `_`, so two distinct servers can produce one
	// name — and the chat dispatches by name, taking the first match, which would run a
	// call against the wrong server.
	it('gives colliding server paths distinct callable names', () => {
		const [first] = registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), { path: 'f/team_a/b' }, [
			TOOLS[0]
		])
		const [second] = registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), { path: 'f/team/a_b' }, [
			TOOLS[0]
		])

		expect(first).not.toBe(second)
		expect(new Set(loadedMcpTools(OWNER).map((t) => t.def.function.name)).size).toBe(2)
	})

	// A search awaits its listing; the conversation can rotate in the meantime. Results
	// registered afterwards would land in the conversation that replaced it.
	it('drops results of a search that finished after the registry was cleared', () => {
		const generation = mcpRegistryGeneration(OWNER)
		forgetLoadedMcpTools(OWNER)

		expect(registerMcpTools(OWNER, generation, server, [TOOLS[0]])).toEqual([undefined])
		expect(loadedMcpTools(OWNER)).toEqual([])
	})

	// Turning a server off while a search is still awaiting its listing leaves nothing
	// registered for the reconcile to find, so the registration itself has to be rejected.
	it('drops results of a search that finished after the connected set changed', () => {
		const generation = mcpRegistryGeneration(OWNER)
		invalidateMcpRegistrations(OWNER)

		expect(registerMcpTools(OWNER, generation, server, [TOOLS[0]])).toEqual([undefined])
		expect(loadedMcpTools(OWNER)).toEqual([])
	})

	// The key is `${server}::${tool}` and a remote tool name is arbitrary — namespaced
	// names are ordinary. Splitting on the last `::` reported a server that does not
	// exist, so the reconcile dropped the tool at the start of every send.
	it('derives the server path from a tool name containing ::', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), server, [
			{ ...TOOLS[0], name: 'issues::list' }
		])

		expect(loadedMcpServers(OWNER).map((s) => s.path)).toEqual(['u/hugo/github_mcp'])
	})

	// The all-owner clear must reject a first search still in flight, which is in
	// neither the registry map nor the per-owner generation map.
	it('drops results of a search that finished after every registry was cleared', () => {
		const generation = mcpRegistryGeneration(OWNER)
		clearMcpToolsCache()

		expect(registerMcpTools(OWNER, generation, server, [TOOLS[0]])).toEqual([undefined])
		expect(loadedMcpTools(OWNER)).toEqual([])
	})

	// A registered call bypasses the listing cache, so an edit made elsewhere has to
	// invalidate the frozen schema — the path alone cannot tell the two apart.
	it('reports the revision each server was registered at', () => {
		registerMcpTools(OWNER, mcpRegistryGeneration(OWNER), { path: server.path, editedAt: 'v1' }, [
			TOOLS[0]
		])

		expect(loadedMcpServers(OWNER)).toEqual([{ path: 'u/hugo/github_mcp', editedAt: 'v1' }])
	})
})

// Dropping the registered tools is how a conversation recovers from a schema the
// provider refuses, and it must not fire on an account problem: a quota error would
// then cost the user every tool they had searched for.
describe('request rejections that withdraw registered tools', () => {
	it('separates a refused body from a refused account', () => {
		expect(isRequestBodyRejection(400)).toBe(true)
		expect(isRequestBodyRejection(422)).toBe(true)
		expect(isRequestBodyRejection(401)).toBe(false)
		expect(isRequestBodyRejection(403)).toBe(false)
		expect(isRequestBodyRejection(429)).toBe(false)
		expect(isRequestBodyRejection(500)).toBe(false)
		// An abort carries no status.
		expect(isRequestBodyRejection(undefined)).toBe(false)
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
	// The schema reaches the model through the registered tool, not inlined here: the
	// result stays a summary so a 10-match search does not also dump 10 schemas into
	// the transcript on top of the tool definitions it just created.
	it('returns compact summaries that name the registered tool', async () => {
		const result = await run('search_mcp_tools', { query: 'issue' })
		expect(result.matches).toEqual([
			{
				server: 'u/hugo/github_mcp',
				tool: 'get_issue',
				call: 'mcp_u_hugo_github_mcp__get_issue',
				description: 'Get details of a GitHub issue',
				mode: 'read'
			}
		])
	})

	// Registering the second server can evict tools the first one just registered, and a
	// `call` name whose tool is gone answers `Unknown tool call` on the next iteration.
	it('advertises no call name for a match a later server evicted', async () => {
		const wide = (name: string) => ({
			name,
			description: 'issue',
			inputSchema: {
				type: 'object',
				properties: { a: { type: 'string', description: 'x'.repeat(7_000) } }
			}
		})
		const servers: McpServer[] = [{ path: 'u/hugo/a_mcp' }, { path: 'u/hugo/b_mcp' }]
		getMcpToolsMock.mockImplementation(({ path }: { path: string }) =>
			Promise.resolve(
				Array.from({ length: 5 }, (_, i) => wide(`${path === 'u/hugo/a_mcp' ? 'a' : 'b'}_${i}`))
			)
		)
		const tool = createMcpTools(OWNER, servers).find(
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

		const registered = new Set(loadedMcpTools(OWNER).map((t) => t.def.function.name))
		expect(registered.size).toBeLessThan(10)
		const advertised = result.matches
			.map((m: { call?: string }) => m.call)
			.filter((c: string | undefined) => c !== undefined)
		expect(advertised.length).toBe(registered.size)
		for (const call of advertised) {
			expect(registered.has(call)).toBe(true)
		}
	})

	it('still returns matches when one server is unreachable', async () => {
		const servers: McpServer[] = [...SERVERS, { path: 'u/hugo/broken_mcp' }]
		getMcpToolsMock.mockImplementation(({ path }: { path: string }) =>
			path === 'u/hugo/broken_mcp'
				? Promise.reject(new Error('connection refused'))
				: Promise.resolve(TOOLS)
		)
		const tool = createMcpTools(OWNER, servers).find(
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
			createMcpTools(OWNER, [{ path: 'u/hugo/github_mcp', editedAt }])
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
