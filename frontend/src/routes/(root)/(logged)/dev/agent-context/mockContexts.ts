import type {
	AgentContext,
	ContextTool
} from '$lib/components/copilot/chat/contextSummary/contextSummary'

const BUILTIN_TOOL_NAMES = [
	'askUserQuestion',
	'cancel_job',
	'close_page',
	'create_artifact',
	'create_folder',
	'delete_workspace_item',
	'deploy_workspace_item',
	'diff',
	'discard_local_draft',
	'edit_script',
	'get_instructions',
	'get_preview_status',
	'get_run',
	'get_schedule_schema',
	'get_trigger_schema',
	'list_artifacts',
	'list_runs',
	'list_workers',
	'list_workspace_items',
	'open_page',
	'open_preview',
	'patch_flow_json',
	'read_artifact',
	'read_dom',
	'read_flow_module_code',
	'read_skill',
	'read_workspace_item',
	'rebase_draft',
	'run_flow',
	'run_script',
	'search_dom',
	'search_resource_types',
	'set_flow_module_code',
	'take_screenshot',
	'test_run_flow',
	'test_run_script',
	'test_run_step',
	'update_user_instructions',
	'write_resource',
	'write_schedule',
	'write_trigger',
	'write_variable'
]

// Read vs write the way the real summary decides it: a tool that only reads is the one
// plan mode admits.
const WRITE_PREFIXES = ['write_', 'delete_', 'deploy_', 'patch_', 'init_', 'test_run_', 'run_']

function tools(names: string[]): ContextTool[] {
	return names.map((name) => ({
		name,
		description: `Mock description of ${name}.`,
		readOnly: !WRITE_PREFIXES.some((p) => name.startsWith(p))
	}))
}

const empty: AgentContext = {
	workspace: 'admins',
	tools: tools(BUILTIN_TOOL_NAMES.slice(0, 38)),
	skills: [],
	mcpServers: [],
	instructions: {}
}

const typical: AgentContext = {
	workspace: 'acme',
	tools: tools(BUILTIN_TOOL_NAMES.slice(0, 38)),
	skills: [
		{
			path: 'f/ai/sql_style',
			name: 'sql_style',
			description: 'How we write SQL against the warehouse',
			enabled: true
		},
		{
			path: 'f/ai/pr_conventions',
			name: 'pr_conventions',
			description: 'Branch naming and PR description format',
			enabled: false
		},
		{
			path: 'f/ai/deploy_checklist',
			name: 'deploy_checklist',
			description: 'Checks to run before deploying a flow to prod',
			enabled: true
		}
	],
	mcpServers: [
		{
			path: 'u/admin/linear',
			name: 'Linear',
			enabled: true
		},
		{
			path: 'u/admin/notion',
			name: 'Notion',
			enabled: false
		}
	],
	instructions: {
		workspace:
			'Scripts go under f/<team>/. Always use the postgres resource f/data/warehouse for analytics queries.',
		user: 'Answer briefly. Prefer Python over TypeScript.'
	}
}

const heavy: AgentContext = {
	workspace: 'platform-prod',
	tools: tools([
		...BUILTIN_TOOL_NAMES,
		'write_app_file',
		'read_app_file',
		'patch_app_file',
		'delete_app_file',
		'init_app',
		'search_app',
		'list_app_runs',
		'get_app_runtime_logs',
		'write_app_runnable',
		'delete_app_runnable',
		'test_run_app_runnable',
		'update_artifact',
		'audit_logs',
		'workspace_settings',
		'resources',
		'variables',
		'schedules',
		'triggers'
	]),
	skills: Array.from({ length: 12 }, (_, i) => ({
		path: `f/ai/skill_${i + 1}`,
		name: [
			'sql_style',
			'pr_conventions',
			'deploy_checklist',
			'incident_runbook',
			'naming_rules',
			'python_deps',
			'retry_policy',
			'slack_alerts',
			'datatable_migrations',
			'hub_usage',
			'secrets_handling',
			'flow_patterns'
		][i],
		description: 'Mock skill description that is long enough to be truncated in a narrow column',
		enabled: i % 4 !== 3
	})),
	mcpServers: [
		{
			path: 'f/mcp/linear',
			name: 'Linear',
			enabled: true
		},
		{
			path: 'f/mcp/github',
			name: 'GitHub',
			enabled: true
		},
		{ path: 'f/mcp/notion', name: 'Notion', enabled: false },
		{ path: 'f/mcp/sentry', name: 'Sentry', enabled: true },
		{ path: 'f/mcp/datadog', name: 'Datadog', enabled: false }
	],
	instructions: {
		workspace: [
			'This is the production platform workspace.',
			'Never deploy directly: stage every change in a fork and ask for review.',
			'Scripts go under f/<team>/, one folder per team.',
			'Use f/data/warehouse for analytics and f/data/app_db for application data.',
			'Every flow must set an error handler pointing to f/ops/alert_oncall.',
			'Schedules run in UTC.'
		].join('\n'),
		user: 'Answer briefly.\nPrefer Python.\nWhen unsure about a resource, ask before creating one.\nI am on the data team.'
	}
}

export const MOCK_CONTEXTS: { id: string; label: string; ctx: AgentContext }[] = [
	{ id: 'empty', label: 'Fresh workspace', ctx: empty },
	{ id: 'typical', label: 'Typical', ctx: typical },
	{ id: 'heavy', label: 'Heavy', ctx: heavy }
]
