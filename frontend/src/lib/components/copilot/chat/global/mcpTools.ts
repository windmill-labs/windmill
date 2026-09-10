import { z } from 'zod'
import { ResourceService, type GetMcpToolsResponse } from '$lib/gen'
import { createToolDef, type Tool } from '../shared'
import { MCP_TOOL_NAME_PREFIX, normalizeToolParameterSchema } from '../toolSchema'
import { enabledMcpPaths } from '$lib/components/mcp/enabledServers'

/**
 * Access to the MCP servers the user has connected (resources of type `mcp`),
 * loaded on demand rather than all at once: a server like GitHub's exposes ~90
 * tools, whose schemas would otherwise sit in the model's context on every
 * iteration.
 *
 * `search_mcp_tools` is the entry point and registers the tools it matched, so
 * from the next iteration each one is a tool of its own carrying the remote's
 * real input schema. Only searched-for tools are ever paid for. The read/write
 * call wrappers remain as a fallback for anything not registered, and take a
 * free-form argument object — which is what the model has to guess into, and
 * why the registered tool is preferred wherever there is one.
 *
 * Registrations do not outlive the page: a reloaded transcript shows the calls it made
 * but no longer offers those tools, and the model searches again to get them back.
 */

type McpToolDef = GetMcpToolsResponse[number]

export type McpServer = { path: string; editedAt?: string }

const MAX_SEARCH_RESULTS = 10
const MAX_DESCRIPTION_CHARS = 200
// A registered tool's schema sits in the model's context for the rest of the
// session, so the set is bounded and the least recently called one is evicted.
const MAX_LOADED_TOOLS = 25
// Both providers cap tool names; OpenAI's 64 is the lower of the two.
const MAX_TOOL_NAME_CHARS = 64
// A registered schema is server-controlled text that rides in every request for the
// rest of the conversation, so it is bounded like every other payload a server
// controls. Past this, the tool is left unregistered and the model reaches it through
// `call_mcp_*` instead — the schema still arrives, but only when a call fails.
const MAX_TOOL_SCHEMA_CHARS = 8_000
// And a ceiling on the set, since the per-tool bound alone would allow 25 large ones —
// far more context than the search indirection exists to save.
const MAX_LOADED_SCHEMA_CHARS = 40_000
// Roomier than the search summary's 200: this is what the model chooses the tool from.
const MAX_TOOL_DESCRIPTION_CHARS = 2_000
const MAX_RESULT_CHARS = 20_000
// A server writes its own error text, and every enabled server can contribute
// one, so search results are capped the same way call results are.
const MAX_SERVER_ERROR_CHARS = 500
// Listing costs a full MCP handshake against a third party, so it is cached —
// but bounded, because `readOnlyHint` decides whether a call needs the user's
// confirmation and must not stay pinned to a stale answer for a whole session.
const TOOLS_CACHE_TTL_MS = 60_000

// Keyed by workspace and revision as well as path: the same path names different
// servers in different workspaces (a fork, most obviously) and, once edited or
// recreated, a different server in the same one. `readOnlyHint` decides whether a
// call needs confirmation, so no listing may outlive the server it describes.
let toolsCache: Record<string, { tools: McpToolDef[]; at: number }> = {}
// Bumped on every clear. A listing in flight when a path is reconnected would
// otherwise land in the fresh cache and answer for the new server with the old
// server's `readOnlyHint` until it expires.
let cacheGeneration = 0

async function loadServerTools(
	workspace: string,
	path: string,
	revision?: string
): Promise<McpToolDef[]> {
	const key = `${workspace}:${path}:${revision ?? ''}`
	const cached = toolsCache[key]
	if (cached && Date.now() - cached.at < TOOLS_CACHE_TTL_MS) {
		return cached.tools
	}
	// A clear while this is in flight means the path may now name a different
	// server, and `readOnlyHint` decides whether a call needs confirmation — so the
	// answer is thrown away and asked again rather than cached or returned.
	for (let attempt = 0; attempt < 2; attempt++) {
		const generation = cacheGeneration
		const tools = await ResourceService.getMcpTools({ workspace, path })
		if (generation === cacheGeneration) {
			toolsCache[key] = { tools, at: Date.now() }
			return tools
		}
	}
	throw new Error(`The tool list for ${path} changed while it was loading. Try again.`)
}

export function clearMcpToolsCache() {
	cacheGeneration++
	toolsCache = {}
	forgetAllLoadedMcpTools()
}

/**
 * Remote tools promoted to first-class chat tools for the current conversation, keyed
 * `${server}::${tool}`. `chatLoop` re-reads its tool list every iteration, so one
 * registered mid-turn is callable on the next, and the whole set is dropped when the
 * conversation rotates.
 *
 * Each entry freezes an input schema and a `readOnlyHint`, which is what
 * `TOOLS_CACHE_TTL_MS` bounds — so entries are dropped by the triggers that drop a
 * listing rather than aged on a timer. `executeTool` re-asserts `read_only` against
 * the live server in between, so a stale hint cannot become an unconfirmed write.
 */
type OwnerRegistry = {
	tools: Map<string, Tool<{}>>
	/** Recency for eviction, kept beside the map rather than as its order: the emitted
	 * tool list carries Anthropic's `cache_control` breakpoint on its last entry, so
	 * reordering it would invalidate the cached prefix for the rest of the turn. */
	lastUsed: Map<string, number>
	/** The server revision each entry was frozen at (see `loadedMcpServers`). */
	editedAt: Map<string, string | undefined>
	counter: number
}

/**
 * Keyed by owner: the docked chat and every warm session runtime each build a manager
 * in GLOBAL mode. One shared map would put a session's registrations into the docked
 * chat's request, and let either one's "New chat" wipe what the other advertised.
 */
const registries = new Map<string, OwnerRegistry>()
/**
 * Bumped whenever an owner's registrations are invalidated. A search awaits a listing
 * before it registers, so it captures this first and drops its results if it moved —
 * otherwise a rotation or disposal during that await is silently undone.
 */
const generations = new Map<string, number>()
/** Registry keys `withdrawMcpToolsAfterRejection` will not let an owner register again. */
const refused = new Map<string, Set<string>>()
/** Bumped by the all-owner clear. Folded into the token below so it also invalidates an
 * owner that has registered nothing yet, and is therefore in neither map. */
let allGeneration = 0

export function mcpRegistryGeneration(owner: string): string {
	return `${allGeneration}:${generations.get(owner) ?? 0}`
}

function invalidateOwner(owner: string) {
	generations.set(owner, (generations.get(owner) ?? 0) + 1)
}

/**
 * Reject the registrations of searches that are still awaiting their listing, without
 * touching what is already registered. Called when the connected server set changes:
 * a search started before a server was turned off has registered nothing yet, so the
 * reconcile over registered tools cannot reach it, and it would otherwise install that
 * server's tools after the fact.
 */
export function invalidateMcpRegistrations(owner: string) {
	invalidateOwner(owner)
}

/**
 * The server half of a `${server}::${tool}` key. Split on the FIRST `::`: a resource
 * path cannot contain `:` (the backend's path validation forbids it), but a remote
 * tool name is arbitrary and namespaced names like `ns::op` are ordinary — splitting
 * on the last one would report a server that does not exist, and the reconcile would
 * then drop that tool at the start of every send.
 */
function serverPathOfKey(key: string): string {
	const at = key.indexOf('::')
	return at === -1 ? key : key.slice(0, at)
}

function registryFor(owner: string): OwnerRegistry {
	let registry = registries.get(owner)
	if (!registry) {
		registry = { tools: new Map(), lastUsed: new Map(), editedAt: new Map(), counter: 0 }
		registries.set(owner, registry)
	}
	return registry
}

function touchLoadedTool(owner: string, key: string) {
	const registry = registryFor(owner)
	if (registry.tools.has(key)) registry.lastUsed.set(key, ++registry.counter)
}

export function loadedMcpTools(owner: string): Tool<{}>[] {
	return [...(registries.get(owner)?.tools.values() ?? [])]
}

/**
 * The servers holding a registered tool, with the revision each was frozen at, for the
 * reconcile in `refreshMcpServers`. The revision matters as much as the path: a
 * registered call bypasses the listing cache, so a connection edited elsewhere would
 * keep running against the schema it had before the edit.
 */
export function loadedMcpServers(owner: string): { path: string; editedAt?: string }[] {
	const registry = registries.get(owner)
	if (!registry) return []
	const seen = new Map<string, string | undefined>()
	for (const key of registry.tools.keys()) {
		const path = serverPathOfKey(key)
		if (!seen.has(path)) seen.set(path, registry.editedAt.get(key))
	}
	return [...seen].map(([path, editedAt]) => ({ path, editedAt }))
}

/**
 * The server a chat-facing tool name belongs to, for marking its transcript row with
 * the provider. Resolved through the registry rather than by parsing the name, which
 * a long path truncates.
 */
export function mcpServerForToolName(owner: string, toolName: string): string | undefined {
	for (const [key, tool] of registries.get(owner)?.tools ?? []) {
		if (tool.def.function.name === toolName) return serverPathOfKey(key)
	}
	return undefined
}

/**
 * Whether a failed chat request was the provider refusing the body rather than the
 * account. A remote schema this provider will not accept refuses every later request
 * in the conversation the same way, so the registered tools are withdrawn on the first
 * of these; auth and quota statuses say nothing about the schemas.
 */
export function isRequestBodyRejection(status: number | undefined): boolean {
	if (status === undefined || status < 400 || status >= 500) return false
	return status !== 401 && status !== 403 && status !== 429
}

/**
 * Drop an owner's registered tools after the provider refused the request, and refuse
 * to register those same tools again for the rest of the conversation. Dropping alone
 * only moves the failure: the model is told to search again, registers the schema that
 * was just refused, and the next request fails the same way. Which schema was at fault
 * is not knowable from the error, so every tool that was loaded goes on the list — each
 * falls back to the free-form wrapper, which is how they were reached before they could
 * be registered at all. A server turned off, edited, or reconnected clears its entries,
 * as does a new conversation.
 */
export function withdrawMcpToolsAfterRejection(owner: string) {
	const keys = [...(registries.get(owner)?.tools.keys() ?? [])]
	forgetLoadedMcpTools(owner)
	if (keys.length > 0) refused.set(owner, new Set(keys))
}

/** Drop one owner's loaded tools, or only those belonging to one server. */
export function forgetLoadedMcpTools(owner: string, serverPath?: string) {
	// Before the empty-registry check: a conversation can rotate while its first search
	// is still awaiting a listing, with nothing registered yet, and that search must
	// still be rejected when it comes back.
	invalidateOwner(owner)
	// Whatever drops a tool also lifts its refusal: a new conversation, and a server
	// turned off or edited, both mean the next listing is worth registering again.
	clearRefused(owner, serverPath)
	const registry = registries.get(owner)
	if (!registry) return
	if (serverPath === undefined) {
		registries.delete(owner)
		return
	}
	const prefix = `${serverPath}::`
	for (const key of [...registry.tools.keys()]) {
		if (key.startsWith(prefix)) {
			registry.tools.delete(key)
			registry.lastUsed.delete(key)
			registry.editedAt.delete(key)
		}
	}
	if (registry.tools.size === 0) registries.delete(owner)
}

function clearRefused(owner: string, serverPath?: string) {
	if (serverPath === undefined) {
		refused.delete(owner)
		return
	}
	const keys = refused.get(owner)
	if (!keys) return
	const prefix = `${serverPath}::`
	for (const key of [...keys]) {
		if (key.startsWith(prefix)) keys.delete(key)
	}
	if (keys.size === 0) refused.delete(owner)
}

/**
 * Drop every owner's loaded tools. Used when a server resource itself changed, which
 * makes the frozen copy every chat holds stale at once — not for one chat rotating.
 */
function forgetAllLoadedMcpTools() {
	allGeneration++
	registries.clear()
	refused.clear()
}

/**
 * Drop one server's listing after the backend refused the read-only assertion it
 * produced. Without this the write call the model is being sent to would be
 * rejected by the same stale `readOnlyHint`, leaving it with nowhere to go until
 * the entry expires.
 */
function forgetServerTools(owner: string, workspace: string, path: string) {
	cacheGeneration++
	const prefix = `${workspace}:${path}:`
	for (const key of Object.keys(toolsCache)) {
		if (key.startsWith(prefix)) delete toolsCache[key]
	}
	forgetLoadedMcpTools(owner, path)
}

/**
 * The `mcp` resources the user turned on for this workspace. Readable is not
 * enough: a shared resource would otherwise put a server the user never chose
 * into every one of their sessions.
 */
export async function loadMcpServers(workspace: string): Promise<McpServer[]> {
	if (!workspace) return []
	// The enabled set is local, so a workspace with nothing turned on is settled
	// without a request — this runs before every send.
	const enabled = enabledMcpPaths(workspace)
	if (enabled.length === 0) return []
	try {
		const resources = await ResourceService.listResource({
			workspace,
			resourceType: 'mcp',
			perPage: 100
		})
		return resources
			.filter((r) => enabled.includes(r.path))
			.map((r) => ({ path: r.path, editedAt: r.edited_at }))
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

function summarizeTool(server: McpServer, tool: McpToolDef, callName?: string) {
	const params = schemaPropertyNames(tool.inputSchema)
	return {
		server: server.path,
		tool: tool.name,
		// A registered tool puts its real schema in front of the model, so bare parameter
		// names are only worth sending for one that is not.
		...(callName ? { call: callName } : params.length > 0 ? { params } : {}),
		description: truncate(tool.description ?? '', MAX_DESCRIPTION_CHARS),
		mode: isReadOnly(tool) ? 'read' : 'write'
	}
}

function errorMessage(e: any): string {
	return e?.body?.error?.message ?? e?.body ?? e?.message ?? String(e)
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
	const tools = await loadServerTools(workspace, server.path, server.editedAt)
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
	owner: string,
	workspace: string,
	server: McpServer,
	tool: McpToolDef,
	args: Record<string, unknown>,
	skippedConfirmation: boolean
): Promise<string> {
	let raw: unknown
	try {
		raw = await ResourceService.callMcpTool({
			workspace,
			path: server.path,
			// The listing this classification came from can predate a resource
			// edited mid-turn, so the backend re-checks the assertion against the
			// server it is about to call.
			requestBody: {
				tool: tool.name,
				arguments: args,
				...(skippedConfirmation ? { read_only: true } : {})
			}
		})
	} catch (e: any) {
		const status = e?.status
		const error = errorMessage(e)
		// The server disagrees with the listing this call was classified from, so the
		// write tool the model is sent to must not be handed the same answer. Matching
		// loosely is safe: the worst a false positive costs is one extra listing.
		if (skippedConfirmation && status === 400 && /read-only/i.test(error)) {
			forgetServerTools(owner, workspace, server.path)
		}
		return bounded({
			success: false,
			...(status ? { status } : {}),
			error,
			// Wrong arguments are the common failure: echo the schema so the model
			// can self-correct on the next call without a separate schema tool.
			...(status >= 400 && status < 500 ? { schema: tool.inputSchema } : {})
		})
	}

	const data = extractResultData(raw)
	// A tool that ran but reported failure comes back as a success with isError set.
	if ((raw as { isError?: boolean })?.isError) {
		return bounded({
			success: false,
			error: typeof data === 'string' ? data : JSON.stringify(data),
			schema: tool.inputSchema
		})
	}

	return bounded({ success: true, data })
}

/**
 * Every result the model sees, within the advertised cap. A server controls both
 * its output *and* its error text, so bounding only the success path would leave
 * a hostile or merely verbose failure free to fill the context window.
 */
function bounded(payload: {
	success: boolean
	data?: unknown
	error?: string
	[k: string]: unknown
}) {
	const full = JSON.stringify(payload)
	if (full.length <= MAX_RESULT_CHARS) return full
	const body = payload.success ? payload.data : payload.error
	const text = typeof body === 'string' ? body : JSON.stringify(body)
	// Measured on the serialized result, not on the text going into it: escaping is
	// the server's to control (a run of backslashes doubles, a control character
	// sextuples), and the ceiling exists to protect the context window.
	let take = MAX_RESULT_CHARS
	let out = ''
	do {
		out = JSON.stringify({
			success: payload.success,
			truncated: true,
			[payload.success ? 'data' : 'error']: text.slice(0, take),
			note: `Truncated to fit ${MAX_RESULT_CHARS} characters. Use filter or pagination parameters to narrow it.`
		})
		take = Math.floor(take / 2)
	} while (out.length > MAX_RESULT_CHARS && take > 0)
	return out
}

/**
 * The search payload under the same ceiling as a call result. Server error text
 * goes first: the matches are what the model asked for.
 */
function boundedSearch(payload: { matches: unknown[]; [k: string]: unknown }): string {
	const { unavailable, ...rest } = payload
	const dropped = Array.isArray(unavailable) ? { unavailableCount: unavailable.length } : {}
	let out = JSON.stringify(payload, null, 2)
	if (out.length <= MAX_RESULT_CHARS) return out
	const matches = [...payload.matches]
	const build = () =>
		JSON.stringify(
			{
				...rest,
				...dropped,
				matches,
				truncated: true,
				note: `Truncated to ${MAX_RESULT_CHARS} characters. Refine the query.`
			},
			null,
			2
		)
	out = build()
	while (out.length > MAX_RESULT_CHARS && matches.length > 0) {
		matches.pop()
		out = build()
	}
	return out
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
 * The read and write call tools differ only in which side of the `readOnlyHint`
 * split they accept, and that check is what keeps a mutating call behind the
 * user's confirmation — building both from one body keeps them from drifting.
 */
function createCallTool(owner: string, servers: McpServer[], mode: 'read' | 'write'): Tool<{}> {
	const isRead = mode === 'read'
	return {
		def: createToolDef(
			callMcpToolSchema,
			isRead ? 'call_mcp_read_tool' : 'call_mcp_write_tool',
			isRead
				? 'Fallback for a read-only tool that search_mcp_tools did not give a `call` name for. Prefer that named tool, which carries the real argument schema; a failed call here returns it.'
				: 'Fallback for a mutating tool that search_mcp_tools did not give a `call` name for; the user is asked to confirm. Prefer that named tool, which carries the real argument schema; a failed call here returns it.'
		),
		showDetails: true,
		...(isRead
			? {}
			: {
					requiresConfirmation: true,
					confirmationMessage: (args: any) => `Call ${args?.tool ?? ''} on ${args?.server ?? ''}`
				}),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = callMcpToolSchema.parse(args)
			// Matched against the connected list rather than taken from the arguments, so
			// what lands on the row is a server the user connected and the transcript can
			// resolve it without re-deciding that. Done before the listing, which needs the
			// server to be reachable — an unreachable one still marks its own failure row.
			const named = servers.find((s) => s.path === parsed.server)
			if (named) toolCallbacks.setToolStatus(toolId, { mcpServer: named.path })
			// Listing is a live call to a third party: a server that has gone away
			// must fail this tool, not the chat loop around it.
			let resolved: Awaited<ReturnType<typeof resolveTool>>
			try {
				resolved = await resolveTool(workspace, servers, parsed.server, parsed.tool)
			} catch (e) {
				resolved = { error: `Could not reach ${parsed.server}: ${errorMessage(e)}` }
			}
			if ('error' in resolved) {
				toolCallbacks.setToolStatus(toolId, { content: resolved.error, error: resolved.error })
				return bounded({ success: false, error: resolved.error })
			}
			if (isReadOnly(resolved.tool) !== isRead) {
				const error = isRead
					? `"${parsed.tool}" is not marked read-only — use call_mcp_write_tool.`
					: `"${parsed.tool}" is read-only — use call_mcp_read_tool (no confirmation needed).`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return bounded({ success: false, error })
			}
			toolCallbacks.setToolStatus(toolId, { content: `Calling ${parsed.tool}...` })
			const result = await executeTool(
				owner,
				workspace,
				resolved.server,
				resolved.tool,
				parsed.arguments ?? {},
				isRead
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
}

function sanitizeToolNamePart(part: string): string {
	return part.replace(/[^a-zA-Z0-9_-]/g, '_')
}

/** djb2, so two long names truncated to the same prefix stay distinct. */
function shortHash(text: string): string {
	let hash = 5381
	for (let i = 0; i < text.length; i++) hash = ((hash * 33) ^ text.charCodeAt(i)) >>> 0
	return hash.toString(36).padStart(7, '0').slice(0, 7)
}

/**
 * The chat-facing name. It carries the server so two servers exposing
 * `list_issues` stay apart, and nothing parses it back: the registry keys on the
 * pair, so truncation only has to stay unique.
 */
function registeredToolName(serverPath: string, toolName: string): string {
	const full = `${MCP_TOOL_NAME_PREFIX}${sanitizeToolNamePart(serverPath)}__${sanitizeToolNamePart(toolName)}`
	if (full.length <= MAX_TOOL_NAME_CHARS) return full
	return `${full.slice(0, MAX_TOOL_NAME_CHARS - 8)}_${shortHash(full)}`
}

/**
 * The name above, disambiguated against what this registry already holds.
 *
 * Sanitizing is lossy — `/` and `_` both become `_`, so `f/team_a/b` and `f/team/a_b`
 * collide — and the chat dispatches by name, taking the first match. Two entries under
 * one name would silently run a call against the wrong server, so the loser is suffixed
 * with a hash of its own key. Hashing the sanitized name cannot fix this: the collision
 * is in that string.
 */
function uniqueRegisteredName(
	registry: OwnerRegistry,
	key: string,
	serverPath: string,
	toolName: string
): string {
	const name = registeredToolName(serverPath, toolName)
	const taken = (candidate: string) => {
		for (const [otherKey, tool] of registry.tools) {
			if (otherKey !== key && tool.def.function.name === candidate) return true
		}
		return false
	}
	if (!taken(name)) return name
	// A remote names its own tools, so the suffixed candidate can be taken too — by a
	// tool that sanitizes to exactly it, or by another key whose hash collided. Salt
	// until it is free; the registry holds MAX_LOADED_TOOLS entries, so this ends.
	const stem = name.slice(0, MAX_TOOL_NAME_CHARS - 8)
	for (let salt = 0; salt <= MAX_LOADED_TOOLS; salt++) {
		const candidate = `${stem}_${shortHash(`${key}#${salt}`)}`
		if (!taken(candidate)) return candidate
	}
	// More collisions than there are entries to collide with: the counter is unique
	// within the registry by construction.
	return `${stem}_${registry.counter}`
}

/**
 * A remote server's `inputSchema`, made safe to send as a provider tool definition.
 * A schema a provider rejects fails the whole completion, not the one tool; the manager
 * drops the registered tools on a rejected request so the next send goes out clean.
 *
 * A guard, not a validator: anything it cannot make sense of collapses to "accepts any
 * object", which costs the model its argument names but keeps the chat alive.
 */
function safeInputSchema(schema: unknown): Record<string, unknown> {
	const empty = { type: 'object', properties: {} }
	if (typeof schema !== 'object' || schema === null || Array.isArray(schema)) return empty
	const { $schema: _dropped, ...rest } = schema as Record<string, unknown>
	const properties =
		typeof rest.properties === 'object' &&
		rest.properties !== null &&
		!Array.isArray(rest.properties)
			? (rest.properties as Record<string, unknown>)
			: {}
	// A name with no property reads as a parameter the model can never satisfy.
	// (`normalizeToolParameterSchema` below is what de-duplicates, at every depth.)
	const required = Array.isArray(rest.required)
		? [
				...new Set(
					rest.required.filter(
						(name): name is string => typeof name === 'string' && Object.hasOwn(properties, name)
					)
				)
			]
		: []
	// Cloned so the normalize pass below, which rewrites nested nodes in place, cannot
	// reach the listing cache these objects came from.
	const safe = structuredClone({ ...rest, type: 'object', properties, required })
	normalizeToolParameterSchema(safe)
	return safe
}

function registrySchemaChars(registry: OwnerRegistry): number {
	let total = 0
	for (const tool of registry.tools.values()) {
		total += JSON.stringify(tool.def.function.parameters ?? {}).length
	}
	return total
}

function evictLoadedTools(registry: OwnerRegistry) {
	const { tools: loadedTools, lastUsed } = registry
	while (
		loadedTools.size > MAX_LOADED_TOOLS ||
		(loadedTools.size > 1 && registrySchemaChars(registry) > MAX_LOADED_SCHEMA_CHARS)
	) {
		let victim: string | undefined
		let victimRank = Infinity
		for (const key of loadedTools.keys()) {
			// Registration counts as a use, so a tool just matched by a search outranks
			// one called earlier. Ranking a fresh registration below every called tool
			// would evict it on arrival once the registry is full, while the search that
			// asked for it still advertised its name.
			const rank = lastUsed.get(key) ?? 0
			if (rank < victimRank) {
				victimRank = rank
				victim = key
			}
		}
		if (victim === undefined) return
		loadedTools.delete(victim)
		lastUsed.delete(victim)
		registry.editedAt.delete(victim)
	}
}

/**
 * Promote remote tools to first-class chat tools, so the model fills a real input
 * schema instead of guessing arguments into a free-form object. Returns the names
 * it registered, in the order given.
 *
 * Each tool carries its own `readOnlyHint`, so the confirmation gate is per tool
 * and the read/write wrappers' "you used the wrong one" failure cannot arise.
 */
export function registerMcpTools(
	owner: string,
	generation: string,
	server: McpServer,
	tools: McpToolDef[]
): (string | undefined)[] {
	// A search awaits a listing, and the conversation can rotate — or the runtime be
	// disposed — while it does. Registering its results afterwards would put the old
	// conversation's tools into the new one, or resurrect a registry nothing can reach.
	if (generation !== mcpRegistryGeneration(owner)) return tools.map(() => undefined)
	const registry = registryFor(owner)
	const refusedKeys = refused.get(owner)
	const names = tools.map((tool) => {
		const key = `${server.path}::${tool.name}`
		// Registering this again is what made the last request fail; the wrapper still
		// reaches it (see `withdrawMcpToolsAfterRejection`).
		if (refusedKeys?.has(key)) return undefined
		const name = uniqueRegisteredName(registry, key, server.path, tool.name)
		const readOnly = isReadOnly(tool)
		const parameters = safeInputSchema(tool.inputSchema)
		if (JSON.stringify(parameters).length > MAX_TOOL_SCHEMA_CHARS) {
			// Left to the wrapper rather than truncated: half a schema would be worse
			// than none, since the model cannot tell which half it is missing.
			return undefined
		}
		// `Map.set` keeps an existing key's position, so re-registering refreshes the
		// schema without moving the emitted list (see `lastUsed`).
		registry.lastUsed.set(key, ++registry.counter)
		registry.editedAt.set(key, server.editedAt)
		registry.tools.set(key, {
			def: {
				type: 'function',
				function: {
					name,
					description: `${truncate(tool.description ?? tool.name, MAX_TOOL_DESCRIPTION_CHARS)}\n(MCP server ${server.path})`,
					parameters
				}
			},
			showDetails: true,
			...(readOnly
				? {}
				: {
						requiresConfirmation: true,
						confirmationMessage: `Call ${tool.name} on ${server.path}`
					}),
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				touchLoadedTool(owner, key)
				toolCallbacks.setToolStatus(toolId, {
					content: `Calling ${tool.name}...`,
					mcpServer: server.path
				})
				const result = await executeTool(owner, workspace, server, tool, args ?? {}, readOnly)
				const ok = JSON.parse(result).success === true
				toolCallbacks.setToolStatus(toolId, {
					content: ok ? `Called ${tool.name}` : `Call to ${tool.name} failed`,
					result,
					...(ok ? {} : { error: `Call to ${tool.name} failed` })
				})
				return result
			}
		})
		return name
	})
	evictLoadedTools(registry)
	// Eviction can drop one of these, and a `call` name the model cannot call is worse
	// than none: without it the summary sends it to the wrapper instead.
	const live = new Set([...registry.tools.values()].map((t) => t.def.function.name))
	return names.map((name) => (name !== undefined && live.has(name) ? name : undefined))
}

/**
 * Built per session from the servers the user connected: with none, the tools
 * are not registered at all, so a workspace without an MCP connection pays no
 * per-iteration schema cost for them.
 */
export function createMcpTools(owner: string, servers: McpServer[]): Tool<{}>[] {
	if (servers.length === 0) return []
	const serverList = servers.map((s) => s.path).join(', ')

	return [
		{
			def: createToolDef(
				searchMcpToolsSchema,
				'search_mcp_tools',
				"Search the tools exposed by the MCP servers connected to this workspace (listed in the system prompt). Each match becomes a callable tool of its own, named by `call` in the result, carrying that tool's real argument schema — call it directly rather than guessing arguments."
			),
			// Openable, so the matches and the `call` names they registered can be read;
			// collapsed once it succeeded, like a call result, since a ten-match search
			// would otherwise take over the transcript.
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const parsed = searchMcpToolsSchema.parse(args)
				// Captured before the listings are awaited: see `registerMcpTools`.
				const generation = mcpRegistryGeneration(owner)
				toolCallbacks.setToolStatus(toolId, { content: 'Searching MCP tools...' })
				const queryTokens = tokenize(parsed.query)
				// One unreachable server must not blank out the others: connecting is a
				// live network call to a third party, so a single failure is expected.
				const unavailable: string[] = []
				const perServer = await Promise.all(
					servers.map(async (server) => {
						try {
							const tools = await loadServerTools(workspace, server.path, server.editedAt)
							return tools.map((tool) => ({ server, tool }))
						} catch (e) {
							unavailable.push(
								`${server.path}: ${errorMessage(e).slice(0, MAX_SERVER_ERROR_CHARS)}`
							)
							return []
						}
					})
				)
				const scored = perServer
					.flat()
					.map((entry) => ({ ...entry, score: scoreTool(entry.tool, queryTokens) }))
					.filter((s) => s.score > 0)
					.sort((a, b) => b.score - a.score || a.tool.name.localeCompare(b.tool.name))

				if (scored.length === 0) {
					const result = boundedSearch({
						matches: [],
						hint: `No tool matched on ${serverList}. Retry with different keywords.`,
						...(unavailable.length > 0 ? { unavailable } : {})
					})
					toolCallbacks.setToolStatus(toolId, { content: 'No matching MCP tool', result })
					return result
				}
				const top = scored.slice(0, MAX_SEARCH_RESULTS)
				// Register the matches, so the next iteration puts each one's real input
				// schema in front of the model instead of leaving it to guess arguments
				// into `call_mcp_*`'s free-form object. Bounded by MAX_SEARCH_RESULTS, and
				// paid only after a search actually matched.
				const callNames = new Map<McpToolDef, string>()
				const perServerMatches = new Map<string, { server: McpServer; tools: McpToolDef[] }>()
				for (const match of top) {
					const entry = perServerMatches.get(match.server.path) ?? {
						server: match.server,
						tools: []
					}
					entry.tools.push(match.tool)
					perServerMatches.set(match.server.path, entry)
				}
				for (const { server, tools } of perServerMatches.values()) {
					const registered = registerMcpTools(owner, generation, server, tools)
					// A tool whose schema was too large to register has no `call` name, and
					// the summary then advertises the wrapper for it instead.
					tools.forEach((tool, i) => {
						const name = registered[i]
						if (name !== undefined) callNames.set(tool, name)
					})
				}
				// Each registration checks its own names against eviction, but registering a
				// later server can evict a name an earlier one just returned. Advertising an
				// evicted name yields `Unknown tool call`; dropping it here sends that match
				// to the wrapper instead.
				const live = new Set(loadedMcpTools(owner).map((t) => t.def.function.name))
				for (const [tool, name] of callNames) {
					if (!live.has(name)) callNames.delete(tool)
				}
				const result = boundedSearch({
					matches: top.map((s) => summarizeTool(s.server, s.tool, callNames.get(s.tool))),
					hint: 'Call each `call` name directly; use the wrappers only for a match without one.',
					...(scored.length > top.length
						? {
								note: `${scored.length - top.length} more match(es) — refine the query to see them.`
							}
						: {}),
					...(unavailable.length > 0 ? { unavailable } : {})
				})
				toolCallbacks.setToolStatus(toolId, {
					content: `Found ${top.length} MCP tool(s) for "${parsed.query}"`,
					result
				})
				return result
			}
		},
		createCallTool(owner, servers, 'read'),
		createCallTool(owner, servers, 'write')
	]
}
