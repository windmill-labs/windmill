/**
 * Mirrors `endpoint_path_policy` / `endpoint_tool_in_scope` in
 * backend/windmill-mcp/src/server/runner.rs. Two endpoint tools are not governed
 * by the `mcp:endpoints:` scope at all, and two more are hidden from tokens
 * confined to specific script paths — without mirroring that here, the scope
 * picker advertises tools the server will never list.
 */

import { mcpEndpointTools } from '$lib/mcpEndpointTools'

export type McpResourceKind = 'script' | 'flow'

export type EndpointPathPolicy =
	/** Runs the script/flow named by the `path` argument. Listed when the token
	 *  grants at least one path of that kind; the endpoint scope is ignored. */
	| { kind: 'runByPath'; resource: McpResourceKind }
	/** Reads/writes a script/flow named by a path argument, itself confined to
	 *  the token's path patterns. Listed by the endpoint scope. */
	| { kind: 'pathArgs'; resource: McpResourceKind }
	/** Takes no checkable path (delete-by-hash) or runs arbitrary code
	 *  (preview); hidden from tokens confined to specific paths. */
	| { kind: 'unconfinable'; resource: McpResourceKind }

export function endpointPathPolicy(name: string): EndpointPathPolicy | undefined {
	switch (name) {
		case 'runScriptByPath':
			return { kind: 'runByPath', resource: 'script' }
		case 'runFlowByPath':
			return { kind: 'runByPath', resource: 'flow' }
		case 'getScriptByPath':
		case 'deleteScriptByPath':
		case 'createScript':
		case 'updateScript':
			return { kind: 'pathArgs', resource: 'script' }
		case 'getFlowByPath':
		case 'deleteFlowByPath':
		case 'createFlow':
		case 'updateFlow':
			return { kind: 'pathArgs', resource: 'flow' }
		case 'deleteScriptByHash':
		case 'runScriptPreviewAndWaitResult':
			return { kind: 'unconfinable', resource: 'script' }
		default:
			return undefined
	}
}

/** The parsed shape of an MCP token's scopes, as `McpScopeConfig` on the backend. */
export type McpScopeState = {
	/** Legacy `mcp:all` — grants every path and endpoint. */
	all: boolean
	/** Legacy `mcp:favorites` — reaches favorites through per-item tools only. */
	favorites: boolean
	scripts: string[]
	flows: string[]
	endpoints: string[]
}

function parseResourceList(resources: string): string[] {
	return resources
		.split(',')
		.map((s) => s.trim())
		.filter((s) => s.length > 0)
}

/** Mirrors `parse_mcp_scopes` so the picker can preview exactly what the server
 *  will make of the scope string it is about to emit. `mcp:hub:` is deliberately
 *  ignored: hub apps select which hub scripts become tools and never affect
 *  endpoint-tool exposure. */
export function parseMcpScopeState(scopes: string[]): McpScopeState {
	const state: McpScopeState = {
		all: false,
		favorites: false,
		scripts: [],
		flows: [],
		endpoints: []
	}
	for (const scope of scopes) {
		if (scope === 'mcp:all') {
			state.all = true
			state.scripts.push('*')
			state.flows.push('*')
			state.endpoints.push('*')
		} else if (scope === 'mcp:favorites') {
			state.favorites = true
		} else if (scope.startsWith('mcp:all:')) {
			const folder = scope.slice('mcp:all:'.length)
			state.scripts.push(folder)
			state.flows.push(folder)
			state.endpoints.push('*')
		} else if (scope.startsWith('mcp:scripts:')) {
			state.scripts.push(...parseResourceList(scope.slice('mcp:scripts:'.length)))
		} else if (scope.startsWith('mcp:flows:')) {
			state.flows.push(...parseResourceList(scope.slice('mcp:flows:'.length)))
		} else if (scope.startsWith('mcp:endpoints:')) {
			state.endpoints.push(...parseResourceList(scope.slice('mcp:endpoints:'.length)))
		}
	}
	return state
}

function patterns(state: McpScopeState, resource: McpResourceKind): string[] {
	return resource === 'script' ? state.scripts : state.flows
}

/** Whether the token grants access to any concrete path of this kind. */
function hasAnyResource(state: McpScopeState, resource: McpResourceKind): boolean {
	return state.all || patterns(state, resource).length > 0
}

/** Whether the token restricts this kind to specific paths. A `*` pattern grants
 *  every path, so it is not a confinement. */
function isPathConfined(state: McpScopeState, resource: McpResourceKind): boolean {
	if (state.all) return false
	const list = patterns(state, resource)
	return list.length > 0 && !list.includes('*')
}

function matchesPattern(name: string, pattern: string): boolean {
	if (pattern === name) return true
	if (!pattern.endsWith('/*')) return false
	const prefix = pattern.slice(0, -2)
	if (!name.startsWith(prefix)) return false
	return name.length === prefix.length || name[prefix.length] === '/'
}

/** Mirrors `is_resource_allowed`: whether a path (or endpoint name) is granted by
 *  any of these patterns, where a pattern is `*`, an exact path, or `<prefix>/*`. */
export function matchesAnyPattern(path: string, patterns: string[]): boolean {
	if (patterns.length === 0) return false
	if (patterns.includes('*')) return true
	return patterns.some((pattern) => matchesPattern(path, pattern))
}

const nonGetEndpoints = new Set(
	mcpEndpointTools.filter((e) => e.method !== 'GET').map((e) => e.name)
)

/** Strip from an `mcp:endpoints:` selection only the names that scope cannot
 *  honor: run-by-path tools, which `isEndpointExposed` gates on the script/flow
 *  scope instead, and — for a read-only token — the non-GET endpoints the server
 *  refuses. Every other entry is preserved, notably a `*` wildcard, which is a
 *  valid endpoint scope that dropping would silently revoke every endpoint tool. */
export function pruneEndpointSelection(selected: string[], readOnly: boolean): string[] {
	return selected.filter(
		(name) =>
			endpointPathPolicy(name)?.kind !== 'runByPath' && !(readOnly && nonGetEndpoints.has(name))
	)
}

/** Whether the server will list this endpoint tool for a token with these scopes. */
export function isEndpointExposed(state: McpScopeState, name: string): boolean {
	const granular = !state.all && !state.favorites
	const endpointAllowed = !granular || matchesAnyPattern(name, state.endpoints)
	const policy = endpointPathPolicy(name)
	switch (policy?.kind) {
		case 'runByPath':
			return hasAnyResource(state, policy.resource)
		case 'unconfinable':
			return endpointAllowed && !isPathConfined(state, policy.resource)
		default:
			return endpointAllowed
	}
}
