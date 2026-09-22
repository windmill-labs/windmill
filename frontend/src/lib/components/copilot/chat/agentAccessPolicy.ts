import type { AgentContextSelection, AgentScopeId } from './contextSummary/selectedEditableFolders'
import { getAgentContextSelection } from './contextSummary/selectedEditableFolders'

export type WorkspaceCapability =
	| 'workspace_context'
	| 'datatable'
	| 'ducklake'
	| 'generic_api'
	| 'create_folder'

export type AgentAccessTarget =
	| { kind: 'workspace_path'; path: string }
	| { kind: 'workspace_capability'; capability: WorkspaceCapability }

function normalizeWorkspacePath(path: string): string {
	return path.replace(/^\$(?:res|var):/, '').replace(/^\/+|\/+$/g, '')
}

export function scopeForWorkspacePath(path: string, username: string): AgentScopeId | undefined {
	const normalized = normalizeWorkspacePath(path)
	const segments = normalized.split('/')
	if (segments.some((segment) => !segment || segment === '.' || segment === '..')) return undefined
	if (segments[0] === 'hub') return undefined
	if (segments[0] === 'f' && segments.length >= 2) return `folder:${segments[1]}`
	if (segments[0] === 'u' && segments.length >= 2) {
		return segments[1] === username ? 'personal' : 'workspace'
	}
	if (segments[0] === 'g' && segments.length >= 2) return 'workspace'
	return undefined
}

export class AgentAccessPolicy {
	readonly #overrides: Set<AgentScopeId>

	constructor(
		readonly username: string,
		readonly selection: AgentContextSelection
	) {
		this.#overrides = new Set(selection.overrides)
	}

	get unrestricted(): boolean {
		return this.selection.baseline === 'selected' && this.#overrides.size === 0
	}

	isScopeSelected(scope: AgentScopeId): boolean {
		const baseline = this.selection.baseline === 'selected'
		return this.#overrides.has(scope) ? !baseline : baseline
	}

	allows(target: AgentAccessTarget): boolean {
		if (target.kind === 'workspace_capability') {
			if (target.capability === 'generic_api') return this.unrestricted
			return this.isScopeSelected('workspace')
		}
		const normalized = normalizeWorkspacePath(target.path)
		if (normalized.startsWith('hub/')) return true
		const scope = scopeForWorkspacePath(normalized, this.username)
		return scope !== undefined && this.isScopeSelected(scope)
	}

	filterPaths<T>(values: T[], pathOf: (value: T) => string): T[] {
		return values.filter((value) => this.allows({ kind: 'workspace_path', path: pathOf(value) }))
	}

	grantCreatedFolder(name: string): void {
		const scope: AgentScopeId = `folder:${name}`
		const baseline = this.selection.baseline === 'selected'
		if (baseline) this.#overrides.delete(scope)
		else this.#overrides.add(scope)
		this.selection.overrides = [...this.#overrides]
	}

	promptSummary(): string {
		if (this.unrestricted) return 'All personal, workspace-wide, and folder scopes are selected.'
		const names = this.selection.overrides.map((scope) =>
			scope.startsWith('folder:') ? `f/${scope.slice('folder:'.length)}` : scope
		)
		if (this.selection.baseline === 'selected') {
			return `All scopes are selected except: ${names.join(', ')}.`
		}
		return names.length
			? `Only these scopes are selected: ${names.join(', ')}.`
			: 'No workspace scopes are selected.'
	}
}

export function createAgentAccessPolicy(workspace: string, username: string): AgentAccessPolicy {
	return new AgentAccessPolicy(username, getAgentContextSelection(workspace))
}

export type AgentAccessHelpers = { agentAccessPolicy?: AgentAccessPolicy }

const ALWAYS_ALLOWED_TOOLS = new Set([
	'askUserQuestion',
	'get_instructions',
	'search_hub_scripts',
	'search_npm_packages',
	'search_docs',
	'read_docs_page',
	'read_file',
	'search_files',
	'create_artifact',
	'list_artifacts',
	'read_artifact',
	'update_artifact',
	'list_artifact_versions',
	'close_page',
	'search_mcp_tools',
	'get_trigger_schema',
	'get_schedule_schema',
	'enter_plan_mode',
	'exit_plan_mode'
])

const WORKSPACE_TOOLS: Partial<Record<string, WorkspaceCapability>> = {
	list_datatables: 'datatable',
	get_datatable_table_schema: 'datatable',
	exec_datatable_sql: 'datatable',
	list_ducklakes: 'ducklake',
	list_workers: 'workspace_context',
	list_runs: 'workspace_context',
	get_run: 'workspace_context',
	cancel_job: 'workspace_context',
	open_page: 'workspace_context',
	search_resource_types: 'workspace_context',
	search_api_endpoints: 'generic_api',
	call_api_get: 'generic_api',
	call_api_endpoint: 'generic_api',
	create_folder: 'create_folder'
}

export function filterToolsForAgentAccess<T extends { def: { function: { name: string } } }>(
	tools: T[],
	policy: AgentAccessPolicy | undefined
): T[] {
	if (!policy) return tools
	return tools.filter((tool) => {
		if (tool.def.function.name === 'update_user_instructions') {
			return policy.isScopeSelected('personal')
		}
		const capability = WORKSPACE_TOOLS[tool.def.function.name]
		return !capability || policy.allows({ kind: 'workspace_capability', capability })
	})
}

const PATH_ARGUMENTS = [
	'path',
	'path_prefix',
	'app_path',
	'resource_path',
	'script_path',
	'flow_path',
	'server'
]

export function toolAccessRejection(
	toolName: string,
	args: unknown,
	helpers: unknown
): { label: string; result: string } | undefined {
	const policy = (helpers as AgentAccessHelpers | undefined)?.agentAccessPolicy
	if (!policy) return undefined
	if (policy.unrestricted) return undefined
	if (ALWAYS_ALLOWED_TOOLS.has(toolName)) return undefined
	if (toolName === 'update_user_instructions') {
		return policy.isScopeSelected('personal')
			? undefined
			: blocked('Personal context is not selected')
	}
	const capability = WORKSPACE_TOOLS[toolName]
	if (capability) {
		return policy.allows({ kind: 'workspace_capability', capability })
			? undefined
			: blocked('Workspace-wide context is not selected')
	}
	if (toolName === 'get_pipeline_graph') {
		const folder = (
			helpers as {
				pipeline?: { getPipelineContext?: () => { folder?: string } }
			}
		)?.pipeline?.getPipelineContext?.().folder
		return folder && policy.allows({ kind: 'workspace_path', path: `f/${folder}` })
			? undefined
			: blocked('The selected context does not include the active pipeline folder')
	}
	const pathArguments = collectPathArguments(args)
	if (toolName === 'open_preview' && (args as Record<string, unknown>)?.kind === 'pipeline') {
		const pipelinePath = (args as Record<string, unknown>).path
		if (typeof pipelinePath === 'string') {
			pathArguments.set(pipelinePath, `f/${pipelinePath}`)
		}
	}
	for (const [shownPath, checkedPath] of pathArguments) {
		if (!policy.allows({ kind: 'workspace_path', path: checkedPath })) {
			return blocked(`The selected context does not include ${shownPath}`)
		}
	}
	if (pathArguments.size > 0) return undefined
	return blocked('This tool has no folder-safe access rule')
}

function collectPathArguments(value: unknown): Map<string, string> {
	const paths = new Map<string, string>()
	const visit = (candidate: unknown, depth: number) => {
		if (!candidate || typeof candidate !== 'object') return
		for (const [key, child] of Object.entries(candidate)) {
			if (
				typeof child === 'string' &&
				PATH_ARGUMENTS.includes(key) &&
				(depth === 0 || key !== 'path')
			) {
				paths.set(child, child)
			} else if (typeof child === 'object') {
				visit(child, depth + 1)
			}
		}
	}
	visit(value, 0)
	return paths
}

function blocked(reason: string) {
	return {
		label: 'Blocked by context selection',
		result: `${reason}. Ask the user to enable the relevant folder or scope in Context before retrying.`
	}
}
