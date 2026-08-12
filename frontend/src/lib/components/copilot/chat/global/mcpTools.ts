import { z } from 'zod'
import { ResourceService, type GetMcpToolsResponse } from '$lib/gen'
import { createToolDef, type Tool } from '../shared'

/**
 * Access to the MCP servers the user has connected (resources of type `mcp`)
 * as three static tools — search, read call, write call — instead of one
 * registered tool per remote tool. A server like GitHub's exposes ~90 tools,
 * whose schemas would otherwise be re-sent on every chat iteration; here only
 * matched summaries enter the model's context, and a full input schema only
 * after a call fails.
 */

type McpToolDef = GetMcpToolsResponse[number]

export type McpServer = { path: string; name: string }

const MAX_SEARCH_RESULTS = 10
const MAX_DESCRIPTION_CHARS = 200
const MAX_RESULT_CHARS = 20_000

let toolsCache: Record<string, McpToolDef[]> = {}
let toolsCacheWorkspace: string | undefined

async function loadServerTools(workspace: string, path: string): Promise<McpToolDef[]> {
	if (toolsCacheWorkspace !== workspace) {
		toolsCache = {}
		toolsCacheWorkspace = workspace
	}
	if (!toolsCache[path]) {
		toolsCache[path] = await ResourceService.getMcpTools({ workspace, path })
	}
	return toolsCache[path]
}

export function clearMcpToolsCache() {
	toolsCache = {}
	toolsCacheWorkspace = undefined
}

/** The `mcp` resources the user can read, i.e. the servers they connected. */
export async function loadMcpServers(workspace: string): Promise<McpServer[]> {
	if (!workspace) return []
	try {
		const resources = await ResourceService.listResource({
			workspace,
			resourceType: 'mcp',
			perPage: 100
		})
		return resources.map((r) => ({
			path: r.path,
			name: r.path.split('/').pop() ?? r.path
		}))
	} catch (e) {
		console.error('Failed to load MCP servers', e)
		return []
	}
}

function tokenize(text: string): string[] {
	return text
		.replace(/([a-z0-9])([A-Z])/g, '$1 $2')
		.toLowerCase()
		.split(/[^a-z0-9]+/)
		.filter((t) => t.length > 1)
}

// Cheap plural-insensitive comparison so "issues" matches "issue" and vice versa.
function tokenMatches(token: string, queryToken: string): boolean {
	const strip = (t: string) => (t.length > 3 && t.endsWith('s') ? t.slice(0, -1) : t)
	return strip(token) === strip(queryToken)
}

// Tool name tokens identify the operation; description tokens only support it.
function scoreTool(tool: McpToolDef, queryTokens: string[]): number {
	const nameTokens = tokenize(tool.name)
	const descTokens = tokenize(tool.description ?? '')
	let score = 0
	for (const qt of queryTokens) {
		if (nameTokens.some((t) => tokenMatches(t, qt))) score += 3
		else if (descTokens.some((t) => tokenMatches(t, qt))) score += 1
	}
	return score
}

// Annotations are hints supplied by the MCP server, so absence must mean
// "assume it writes": treating an unannotated tool as read-only would let it
// run without the user's confirmation.
function isReadOnly(tool: McpToolDef): boolean {
	return tool.annotations?.readOnlyHint === true
}

function schemaPropertyNames(schema: unknown): string[] {
	const properties = (schema as { properties?: Record<string, unknown> } | null | undefined)
		?.properties
	return properties ? Object.keys(properties) : []
}

function truncate(text: string, max: number): string {
	return text.length > max ? text.slice(0, max) + '…' : text
}

function summarizeTool(server: McpServer, tool: McpToolDef) {
	const params = schemaPropertyNames(tool.inputSchema)
	return {
		server: server.path,
		tool: tool.name,
		description: truncate(tool.description ?? '', MAX_DESCRIPTION_CHARS),
		mode: isReadOnly(tool) ? 'read' : 'write',
		...(params.length > 0 ? { params } : {})
	}
}

async function resolveTool(
	workspace: string,
	servers: McpServer[],
	serverPath: string,
	toolName: string
): Promise<{ server: McpServer; tool: McpToolDef } | { error: string }> {
	const server = servers.find((s) => s.path === serverPath)
	if (!server) {
		return {
			error: `Unknown MCP server "${serverPath}". Connected servers: ${servers.map((s) => s.path).join(', ')}`
		}
	}
	const tools = await loadServerTools(workspace, server.path)
	const tool = tools.find((t) => t.name === toolName)
	if (!tool) {
		return {
			error: `Unknown tool "${toolName}" on ${server.path}. Use search_mcp_tools to find the tool name.`
		}
	}
	return { server, tool }
}

/** Flatten the MCP content blocks into the text the model can act on. */
function extractResultData(result: unknown): unknown {
	const content = (result as { content?: unknown })?.content
	if (Array.isArray(content)) {
		const texts = content
			.filter((c) => (c as { type?: string })?.type === 'text')
			.map((c) => (c as { text?: string }).text ?? '')
		if (texts.length === content.length) return texts.join('\n')
	}
	const structured = (result as { structuredContent?: unknown })?.structuredContent
	return structured ?? content ?? result
}

async function executeTool(
	workspace: string,
	server: McpServer,
	tool: McpToolDef,
	args: Record<string, unknown>
): Promise<string> {
	const response = await fetch(
		`/api/w/${encodeURIComponent(workspace)}/resources/mcp_call_tool/${server.path}`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ tool: tool.name, arguments: args })
		}
	)
	const raw = response.headers.get('content-type')?.includes('application/json')
		? await response.json()
		: await response.text()

	if (!response.ok) {
		return JSON.stringify({
			success: false,
			status: response.status,
			error: typeof raw === 'string' ? raw : JSON.stringify(raw),
			// Wrong arguments are the common failure — echo the schema so the model
			// can self-correct on the next call without a separate schema tool.
			...(response.status >= 400 && response.status < 500 ? { schema: tool.inputSchema } : {})
		})
	}

	const data = extractResultData(raw)
	// A tool that ran but reported failure comes back as a 200 with isError set.
	if ((raw as { isError?: boolean })?.isError) {
		return JSON.stringify({
			success: false,
			error: typeof data === 'string' ? data : JSON.stringify(data),
			schema: tool.inputSchema
		})
	}

	const result = JSON.stringify({ success: true, data })
	if (result.length <= MAX_RESULT_CHARS) return result
	return JSON.stringify({
		success: true,
		truncated: true,
		data: (typeof data === 'string' ? data : JSON.stringify(data)).slice(0, MAX_RESULT_CHARS),
		note: `Result truncated to ${MAX_RESULT_CHARS} characters. Use filter or pagination parameters to narrow it.`
	})
}

const searchMcpToolsSchema = z.object({
	query: z
		.string()
		.describe("Keywords matched against the connected servers' tool names and descriptions")
})

const callMcpToolSchema = z.object({
	server: z.string().describe('MCP server resource path as returned by search_mcp_tools'),
	tool: z.string().describe('Tool name as returned by search_mcp_tools'),
	arguments: z
		.record(z.string(), z.any())
		.optional()
		.describe('Tool arguments, keyed by parameter name')
})

/**
 * Built per session from the servers the user connected so the descriptions can
 * name them: without that the model would have to search blind to discover that
 * a GitHub connection exists at all.
 */
export function createMcpTools(servers: McpServer[]): Tool<{}>[] {
	if (servers.length === 0) return []
	const serverList = servers.map((s) => s.path).join(', ')

	return [
		{
			def: createToolDef(
				searchMcpToolsSchema,
				'search_mcp_tools',
				`Search the tools exposed by the MCP servers connected to this workspace (${serverList}). Returns server + tool names to pass to call_mcp_read_tool or call_mcp_write_tool.`
			),
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = searchMcpToolsSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: 'Searching MCP tools...' })
				const queryTokens = tokenize(parsed.query)
				const perServer = await Promise.all(
					servers.map(async (server) => {
						const tools = await loadServerTools(workspace, server.path)
						return tools.map((tool) => ({ server, tool }))
					})
				)
				const scored = perServer
					.flat()
					.map((entry) => ({ ...entry, score: scoreTool(entry.tool, queryTokens) }))
					.filter((s) => s.score > 0)
					.sort((a, b) => b.score - a.score || a.tool.name.localeCompare(b.tool.name))

				if (scored.length === 0) {
					const result = JSON.stringify(
						{
							matches: [],
							hint: `No tool matched on ${serverList}. Retry with different keywords.`
						},
						null,
						2
					)
					toolCallbacks.setToolStatus(toolId, { content: 'No matching MCP tool', result })
					return result
				}
				const top = scored.slice(0, MAX_SEARCH_RESULTS)
				const result = JSON.stringify(
					{
						matches: top.map((s) => summarizeTool(s.server, s.tool)),
						...(scored.length > top.length
							? {
									note: `${scored.length - top.length} more match(es) — refine the query to see them.`
								}
							: {})
					},
					null,
					2
				)
				toolCallbacks.setToolStatus(toolId, {
					content: `Found ${top.length} MCP tool(s) for "${parsed.query}"`,
					result
				})
				return result
			}
		},
		{
			def: createToolDef(
				callMcpToolSchema,
				'call_mcp_read_tool',
				'Call a read-only tool on a connected MCP server. Use search_mcp_tools first to find the server and tool names; a failed call returns the tool argument schema.'
			),
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = callMcpToolSchema.parse(args)
				const resolved = await resolveTool(workspace, servers, parsed.server, parsed.tool)
				if ('error' in resolved) {
					toolCallbacks.setToolStatus(toolId, { content: resolved.error, error: resolved.error })
					return JSON.stringify({ success: false, error: resolved.error })
				}
				if (!isReadOnly(resolved.tool)) {
					const error = `"${parsed.tool}" is not marked read-only — use call_mcp_write_tool.`
					toolCallbacks.setToolStatus(toolId, { content: error, error })
					return JSON.stringify({ success: false, error })
				}
				toolCallbacks.setToolStatus(toolId, { content: `Calling ${parsed.tool}...` })
				const result = await executeTool(
					workspace,
					resolved.server,
					resolved.tool,
					parsed.arguments ?? {}
				)
				const ok = JSON.parse(result).success === true
				toolCallbacks.setToolStatus(toolId, {
					content: ok ? `Called ${parsed.tool}` : `Call to ${parsed.tool} failed`,
					result,
					...(ok ? {} : { error: `Call to ${parsed.tool} failed` })
				})
				return result
			}
		},
		{
			def: createToolDef(
				callMcpToolSchema,
				'call_mcp_write_tool',
				'Call a tool that modifies data on a connected MCP server; the user is asked to confirm. Use search_mcp_tools first to find the server and tool names; a failed call returns the tool argument schema.'
			),
			requiresConfirmation: true,
			confirmationMessage: (args) => `Call ${args?.tool ?? ''} on ${args?.server ?? ''}`,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = callMcpToolSchema.parse(args)
				const resolved = await resolveTool(workspace, servers, parsed.server, parsed.tool)
				if ('error' in resolved) {
					toolCallbacks.setToolStatus(toolId, { content: resolved.error, error: resolved.error })
					return JSON.stringify({ success: false, error: resolved.error })
				}
				if (isReadOnly(resolved.tool)) {
					const error = `"${parsed.tool}" is read-only — use call_mcp_read_tool (no confirmation needed).`
					toolCallbacks.setToolStatus(toolId, { content: error, error })
					return JSON.stringify({ success: false, error })
				}
				toolCallbacks.setToolStatus(toolId, { content: `Calling ${parsed.tool}...` })
				const result = await executeTool(
					workspace,
					resolved.server,
					resolved.tool,
					parsed.arguments ?? {}
				)
				const ok = JSON.parse(result).success === true
				toolCallbacks.setToolStatus(toolId, {
					content: ok ? `Called ${parsed.tool}` : `Call to ${parsed.tool} failed`,
					result,
					...(ok ? {} : { error: `Call to ${parsed.tool} failed` })
				})
				return result
			}
		}
	]
}
