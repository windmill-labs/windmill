import { z } from 'zod'
import { McpService, type EndpointTool } from '$lib/gen'
import { createToolDef, type Tool } from '../shared'

/**
 * Generic access to the backend's MCP endpoint catalog (the endpoints marked
 * `x-mcp-tool` in openapi.yaml) as three small static tools — search, GET
 * call, mutating call — instead of one registered tool per endpoint. Search
 * results carry parameter names and usage instructions; full parameter schemas
 * enter the model's context only when a call fails validation. This keeps the
 * per-iteration tool-schema cost constant.
 */

// Endpoints whose job a dedicated global tool already does, plus the variable
// read endpoints. Hidden from search and refused at call time: the authoring
// and delete ones would bypass the draft lifecycle (conflict detection,
// explicit deploy, draft cleanup on delete), the variable reads would expose
// variable values to the model (getVariable even decrypts secrets by default),
// and the rest are exact duplicates that would fragment behavior across two
// code paths.
const COVERED_ENDPOINTS: Record<string, string> = {
	getVariable: 'read_workspace_item (variable values are never readable in chat)',
	listVariable: 'list_workspace_items (variable values are never readable in chat)',
	// The item read/list endpoints return deployed state only, blind to the user's
	// drafts; read_workspace_item / list_workspace_items merge drafts, and for
	// flows return the compact JSON that patch_flow_json matches against.
	getScriptByPath: 'read_workspace_item (it reads your draft when one exists)',
	getFlowByPath: 'read_workspace_item (it reads your draft when one exists)',
	getResource: 'read_workspace_item (it reads your draft when one exists)',
	getSchedule: 'read_workspace_item (it reads your draft when one exists)',
	listScripts: 'list_workspace_items (it includes your drafts)',
	listFlows: 'list_workspace_items (it includes your drafts)',
	listResource: 'list_workspace_items (it includes your drafts)',
	listSchedules: 'list_workspace_items (it includes your drafts)',
	deleteScriptByPath: 'delete_workspace_item',
	deleteScriptByHash: 'delete_workspace_item',
	deleteFlowByPath: 'delete_workspace_item',
	deleteSchedule: 'delete_workspace_item',
	deleteVariable: 'delete_workspace_item',
	deleteResource: 'delete_workspace_item',
	createScript: 'write_script',
	createFlow: 'write_flow',
	updateFlow: 'patch_flow_json or write_flow',
	createApp: 'init_app and the app draft tools',
	updateApp: 'write_app_file / write_app_runnable',
	createVariable: 'write_variable',
	updateVariable: 'write_variable',
	createResource: 'write_resource',
	updateResource: 'write_resource',
	createSchedule: 'write_schedule',
	updateSchedule: 'write_schedule',
	searchDocs: 'search_docs',
	readDocsPage: 'read_docs_page',
	listJobs: 'list_runs',
	getJobLogs: 'get_job_logs',
	runScriptPreviewAndWaitResult: 'test_run_script'
}

const MAX_SEARCH_RESULTS = 10
const MAX_DESCRIPTION_CHARS = 200
const MAX_INSTRUCTIONS_CHARS = 400
const MAX_RESULT_CHARS = 20_000

let catalogCache: { workspace: string; endpoints: EndpointTool[] } | undefined

async function loadCatalog(workspace: string): Promise<EndpointTool[]> {
	if (catalogCache?.workspace !== workspace) {
		const endpoints = await McpService.listMcpTools({ workspace })
		catalogCache = { workspace, endpoints }
	}
	return catalogCache.endpoints
}

export function clearApiCatalogCache() {
	catalogCache = undefined
}

function tokenize(text: string): string[] {
	return text
		.replace(/([a-z0-9])([A-Z])/g, '$1 $2')
		.toLowerCase()
		.split(/[^a-z0-9]+/)
		.filter((t) => t.length > 1)
}

// Cheap plural-insensitive comparison so "workers" matches "worker" and vice versa.
function tokenMatches(token: string, queryToken: string): boolean {
	const strip = (t: string) => (t.length > 3 && t.endsWith('s') ? t.slice(0, -1) : t)
	return strip(token) === strip(queryToken)
}

function pathSegments(path: string): string[] {
	return path.split('/').filter((seg) => seg && seg !== 'w' && !seg.startsWith('{'))
}

// Name and path tokens identify the operation; description tokens only support it.
function scoreEndpoint(endpoint: EndpointTool, queryTokens: string[]): number {
	const nameTokens = [...tokenize(endpoint.name), ...pathSegments(endpoint.path).flatMap(tokenize)]
	const descTokens = tokenize(`${endpoint.description} ${endpoint.instructions}`)
	let score = 0
	for (const qt of queryTokens) {
		if (nameTokens.some((t) => tokenMatches(t, qt))) score += 3
		else if (descTokens.some((t) => tokenMatches(t, qt))) score += 1
	}
	return score
}

function schemaPropertyNames(schema: unknown): string[] {
	const properties = (schema as { properties?: Record<string, unknown> } | null | undefined)
		?.properties
	return properties ? Object.keys(properties).filter((k) => k !== 'workspace') : []
}

function truncate(text: string, max: number): string {
	return text.length > max ? text.slice(0, max) + '…' : text
}

function summarizeEndpoint(endpoint: EndpointTool) {
	const params = [
		...schemaPropertyNames(endpoint.path_params_schema),
		...schemaPropertyNames(endpoint.query_params_schema)
	]
	const bodyParams = schemaPropertyNames(endpoint.body_schema)
	const instructions = endpoint.instructions.trim()
	return {
		name: endpoint.name,
		endpoint: `${endpoint.method.toUpperCase()} ${endpoint.path}`,
		description: truncate(endpoint.description, MAX_DESCRIPTION_CHARS),
		...(instructions ? { instructions: truncate(instructions, MAX_INSTRUCTIONS_CHARS) } : {}),
		...(params.length > 0 ? { params } : {}),
		...(bodyParams.length > 0 ? { body_params: bodyParams } : {})
	}
}

function endpointSchemaHelp(endpoint: EndpointTool) {
	return {
		name: endpoint.name,
		endpoint: `${endpoint.method.toUpperCase()} ${endpoint.path}`,
		path_params_schema: endpoint.path_params_schema ?? undefined,
		query_params_schema: endpoint.query_params_schema ?? undefined,
		body_schema: endpoint.body_schema ?? undefined
	}
}

async function resolveEndpoint(
	workspace: string,
	name: string
): Promise<{ endpoint: EndpointTool } | { error: string }> {
	const covered = COVERED_ENDPOINTS[name]
	if (covered) {
		return { error: `"${name}" is covered by the dedicated ${covered} tool — use it instead.` }
	}
	const catalog = await loadCatalog(workspace)
	const endpoint = catalog.find((e) => e.name === name)
	if (!endpoint) {
		return {
			error: `Unknown endpoint "${name}". Use search_api_endpoints to find the endpoint name.`
		}
	}
	return { endpoint }
}

async function executeEndpoint(
	endpoint: EndpointTool,
	workspace: string,
	params: Record<string, unknown>,
	body?: unknown
): Promise<string> {
	let url = `/api${endpoint.path.replace('{workspace}', encodeURIComponent(workspace))}`
	const queryParams = new URLSearchParams()
	for (const [key, value] of Object.entries(params)) {
		if (value === undefined || value === null) continue
		if (url.includes(`{${key}}`)) {
			url = url.replace(`{${key}}`, encodeURIComponent(String(value)))
		} else {
			queryParams.append(key, String(value))
		}
	}
	const unresolved = [...url.matchAll(/\{([^}]+)\}/g)].map((m) => m[1])
	if (unresolved.length > 0) {
		return JSON.stringify({
			success: false,
			error: `Missing required path parameter(s): ${unresolved.join(', ')}`,
			schema: endpointSchemaHelp(endpoint)
		})
	}
	const search = queryParams.toString()
	if (search) url += `?${search}`

	const method = endpoint.method.toUpperCase()
	const fetchOptions: RequestInit = { method }
	if (body !== undefined && method !== 'GET') {
		fetchOptions.headers = { 'Content-Type': 'application/json' }
		fetchOptions.body = JSON.stringify(body)
	}

	const response = await fetch(url, fetchOptions)
	const raw = response.headers.get('content-type')?.includes('application/json')
		? await response.json()
		: await response.text()
	if (!response.ok) {
		return JSON.stringify({
			success: false,
			status: response.status,
			error: typeof raw === 'string' ? raw : JSON.stringify(raw),
			// 4xx usually means wrong arguments — echo the schema so the model can
			// self-correct in the next call without a separate schema-fetch tool.
			...(response.status >= 400 && response.status < 500
				? { schema: endpointSchemaHelp(endpoint) }
				: {})
		})
	}
	const result = JSON.stringify({ success: true, data: raw })
	if (result.length <= MAX_RESULT_CHARS) return result
	return JSON.stringify({
		success: true,
		truncated: true,
		data: (typeof raw === 'string' ? raw : JSON.stringify(raw)).slice(0, MAX_RESULT_CHARS),
		note: `Result truncated to ${MAX_RESULT_CHARS} characters. Use filter or pagination parameters to narrow it.`
	})
}

const searchApiEndpointsSchema = z.object({
	query: z
		.string()
		.describe(
			'Keywords matched against endpoint names, paths, and descriptions (e.g. "workers", "queue", "run flow"). Jobs are called "runs" in the UI.'
		)
})

const callApiGetSchema = z.object({
	name: z.string().describe('Endpoint name as returned by search_api_endpoints'),
	params: z
		.record(z.string(), z.any())
		.optional()
		.describe(
			'Path and query parameter values, keyed by parameter name. The workspace parameter is filled automatically.'
		)
})

const callApiEndpointSchema = z.object({
	name: z.string().describe('Endpoint name as returned by search_api_endpoints'),
	params: z
		.record(z.string(), z.any())
		.optional()
		.describe(
			'Path and query parameter values, keyed by parameter name. The workspace parameter is filled automatically.'
		),
	body: z
		.record(z.string(), z.any())
		.optional()
		.describe('JSON request body, when the endpoint takes one')
})

export const apiCatalogTools: Tool<{}>[] = [
	{
		def: createToolDef(
			searchApiEndpointsSchema,
			'search_api_endpoints',
			'Search the Windmill REST API endpoint catalog for operations no dedicated tool covers (workers, queue state, job details, running deployed items, deletions, ...). Returns endpoint names to pass to call_api_get or call_api_endpoint.'
		),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = searchApiEndpointsSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: 'Searching API endpoints...' })
			const catalog = await loadCatalog(workspace)
			const queryTokens = tokenize(parsed.query)
			const available = catalog.filter((e) => !COVERED_ENDPOINTS[e.name])
			const scored = available
				.map((endpoint) => ({ endpoint, score: scoreEndpoint(endpoint, queryTokens) }))
				.filter((s) => s.score > 0)
				.sort((a, b) => b.score - a.score || a.endpoint.name.localeCompare(b.endpoint.name))
			const coveredHits = catalog
				.filter((e) => COVERED_ENDPOINTS[e.name] && scoreEndpoint(e, queryTokens) > 0)
				.map((e) => `${e.name} → use ${COVERED_ENDPOINTS[e.name]}`)

			if (scored.length === 0) {
				const categories = [...new Set(available.map((e) => pathSegments(e.path)[0]))].sort()
				const result = JSON.stringify(
					{
						matches: [],
						hint: `No endpoint matched. Available endpoint categories: ${categories.join(', ')}. Retry with different keywords, or use a dedicated tool if one covers the need.`,
						...(coveredHits.length > 0 ? { covered_by_dedicated_tools: coveredHits } : {})
					},
					null,
					2
				)
				toolCallbacks.setToolStatus(toolId, { content: 'No matching API endpoint', result })
				return result
			}
			const top = scored.slice(0, MAX_SEARCH_RESULTS)
			const result = JSON.stringify(
				{
					matches: top.map((s) => summarizeEndpoint(s.endpoint)),
					...(scored.length > top.length
						? {
								note: `${scored.length - top.length} more match(es) — refine the query to see them.`
							}
						: {}),
					...(coveredHits.length > 0 ? { covered_by_dedicated_tools: coveredHits } : {})
				},
				null,
				2
			)
			toolCallbacks.setToolStatus(toolId, {
				content: `Found ${top.length} API endpoint(s) for "${parsed.query}"`,
				result
			})
			return result
		}
	},
	{
		def: createToolDef(
			callApiGetSchema,
			'call_api_get',
			'Call a read-only GET endpoint from the API catalog by name. Use search_api_endpoints first to find the endpoint name; a failed call returns the parameter schema.'
		),
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = callApiGetSchema.parse(args)
			const resolved = await resolveEndpoint(workspace, parsed.name)
			if ('error' in resolved) {
				toolCallbacks.setToolStatus(toolId, { content: resolved.error, error: resolved.error })
				return JSON.stringify({ success: false, error: resolved.error })
			}
			if (resolved.endpoint.method.toUpperCase() !== 'GET') {
				const error = `"${parsed.name}" is a ${resolved.endpoint.method.toUpperCase()} endpoint — use call_api_endpoint for mutating calls.`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			toolCallbacks.setToolStatus(toolId, { content: `Calling ${parsed.name}...` })
			const result = await executeEndpoint(resolved.endpoint, workspace, parsed.params ?? {})
			const ok = JSON.parse(result).success === true
			toolCallbacks.setToolStatus(toolId, {
				content: ok ? `Called ${parsed.name}` : `Call to ${parsed.name} failed`,
				result,
				...(ok ? {} : { error: `Call to ${parsed.name} failed` })
			})
			return result
		}
	},
	{
		def: createToolDef(
			callApiEndpointSchema,
			'call_api_endpoint',
			'Call a mutating (POST/PUT/PATCH/DELETE) endpoint from the API catalog by name; the user is asked to confirm. Use search_api_endpoints first to find the endpoint name; a failed call returns the parameter schema.'
		),
		requiresConfirmation: true,
		confirmationMessage: (args) => `Call API endpoint ${args?.name ?? ''}`,
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = callApiEndpointSchema.parse(args)
			const resolved = await resolveEndpoint(workspace, parsed.name)
			if ('error' in resolved) {
				toolCallbacks.setToolStatus(toolId, { content: resolved.error, error: resolved.error })
				return JSON.stringify({ success: false, error: resolved.error })
			}
			if (resolved.endpoint.method.toUpperCase() === 'GET') {
				const error = `"${parsed.name}" is a GET endpoint — use call_api_get (no confirmation needed).`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			toolCallbacks.setToolStatus(toolId, { content: `Calling ${parsed.name}...` })
			const result = await executeEndpoint(
				resolved.endpoint,
				workspace,
				parsed.params ?? {},
				parsed.body
			)
			const ok = JSON.parse(result).success === true
			toolCallbacks.setToolStatus(toolId, {
				content: ok ? `Called ${parsed.name}` : `Call to ${parsed.name} failed`,
				result,
				...(ok ? {} : { error: `Call to ${parsed.name} failed` })
			})
			return result
		}
	}
]
