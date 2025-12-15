import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createSearchHubScriptsTool, createToolDef, type Tool } from '../shared'
import { FlowService, ScriptService, type Flow, type Script } from '$lib/gen'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'

// Backend runnable types
export type BackendRunnableType = 'script' | 'flow' | 'hubscript' | 'inline'

export interface InlineScript {
	language: 'bun' | 'python3'
	content: string
}

export interface BackendRunnable {
	name: string
	type: BackendRunnableType
	staticInputs?: Record<string, any>
	// For inline scripts
	inlineScript?: InlineScript
	// For workspace scripts/flows and hub scripts
	path?: string
}

export interface AppFiles {
	frontend: Record<string, string>
	backend: Record<string, BackendRunnable>
}

// Lint result types
export interface LintMessages {
	/** Error/warning messages from frontend files */
	frontend: Record<string, string[]>
	/** Error/warning messages from backend runnables */
	backend: Record<string, string[]>
}

export interface LintResult {
	/** Total count of errors across all files */
	errorCount: number
	/** Total count of warnings across all files */
	warningCount: number
	/** Error messages grouped by source type */
	errors: LintMessages
	/** Warning messages grouped by source type */
	warnings: LintMessages
}

/** Context about the currently selected file or runnable in the app editor */
export interface SelectedContext {
	/** Type of selection: 'frontend' for frontend files, 'backend' for backend runnables, or 'none' if nothing is selected */
	type: 'frontend' | 'backend' | 'none'
	/** The path of the selected frontend file (when type is 'frontend') */
	frontendPath?: string
	/** The key of the selected backend runnable (when type is 'backend') */
	backendKey?: string
	// Future: text selection within the file
	// textSelection?: { startLine: number; endLine: number; startColumn: number; endColumn: number }
}

export interface AppAIChatHelpers {
	// Frontend file operations
	listFrontendFiles: () => string[]
	getFrontendFile: (path: string) => string | undefined
	getFrontendFiles: () => Record<string, string>
	/** Sets a frontend file and returns lint results */
	setFrontendFile: (path: string, content: string) => LintResult
	deleteFrontendFile: (path: string) => void
	// Backend runnable operations
	listBackendRunnables: () => { key: string; name: string }[]
	getBackendRunnable: (key: string) => BackendRunnable | undefined
	getBackendRunnables: () => Record<string, BackendRunnable>
	/** Sets a backend runnable, switches UI to it, waits for Monaco to analyze, and returns lint results */
	setBackendRunnable: (key: string, runnable: BackendRunnable) => Promise<LintResult>
	deleteBackendRunnable: (key: string) => void
	// Combined view
	getFiles: () => AppFiles
	getSelectedContext: () => SelectedContext
	snapshot: () => number
	revertToSnapshot: (id: number) => void
	// Linting
	/** Lint all frontend files and backend runnables, returns errors and warnings */
	lint: () => LintResult
}

// ============= Utility =============

/** Memoize a factory function - the factory is only called once, on first access */
const memo = <T>(factory: () => T): (() => T) => {
	let cached: T | undefined
	return () => (cached ??= factory())
}

// ============= Frontend File Tools =============

const getListFrontendFilesSchema = memo(() => z.object({}))
const getListFrontendFilesToolDef = memo(() =>
	createToolDef(
		getListFrontendFilesSchema(),
		'list_frontend_files',
		'List all frontend file paths in the raw app. Returns an array of file paths without content. Use this for overview, then get specific files.'
	)
)

const getGetFrontendFileSchema = memo(() =>
	z.object({
		path: z
			.string()
			.describe('The path of the frontend file to get (e.g., /index.tsx, /styles.css)')
	})
)
const getGetFrontendFileToolDef = memo(() =>
	createToolDef(
		getGetFrontendFileSchema(),
		'get_frontend_file',
		'Get the content of a specific frontend file by path. Use this to inspect individual files.'
	)
)

const getGetFrontendFilesSchema = memo(() => z.object({}))
const getGetFrontendFilesToolDef = memo(() =>
	createToolDef(
		getGetFrontendFilesSchema(),
		'get_frontend_files',
		'Get all frontend files in the raw app. Returns a record of file paths to their content. Use list_frontend_files + get_frontend_file for large apps.'
	)
)

const getSetFrontendFileSchema = memo(() =>
	z.object({
		path: z
			.string()
			.describe(
				'The path of the frontend file to create or update (e.g., /index.tsx, /styles.css)'
			),
		content: z.string().describe('The content of the file')
	})
)
const getSetFrontendFileToolDef = memo(() =>
	createToolDef(
		getSetFrontendFileSchema(),
		'set_frontend_file',
		'Create or update a frontend file in the raw app. Returns lint diagnostics (errors and warnings).'
	)
)

const getDeleteFrontendFileSchema = memo(() =>
	z.object({
		path: z.string().describe('The path of the frontend file to delete')
	})
)
const getDeleteFrontendFileToolDef = memo(() =>
	createToolDef(
		getDeleteFrontendFileSchema(),
		'delete_frontend_file',
		'Delete a frontend file from the raw app'
	)
)

// ============= Backend Runnable Tools =============

const getListBackendRunnablesSchema = memo(() => z.object({}))
const getListBackendRunnablesToolDef = memo(() =>
	createToolDef(
		getListBackendRunnablesSchema(),
		'list_backend_runnables',
		'List all backend runnable keys and names in the raw app. Returns an array without full content. Use this for overview, then get specific runnables.'
	)
)

const getGetBackendRunnableSchema = memo(() =>
	z.object({
		key: z.string().describe('The key of the backend runnable to get')
	})
)
const getGetBackendRunnableToolDef = memo(() =>
	createToolDef(
		getGetBackendRunnableSchema(),
		'get_backend_runnable',
		'Get the full configuration of a specific backend runnable by key. Use this to inspect individual runnables.'
	)
)

const getGetBackendRunnablesSchema = memo(() => z.object({}))
const getGetBackendRunnablesToolDef = memo(() =>
	createToolDef(
		getGetBackendRunnablesSchema(),
		'get_backend_runnables',
		'Get all backend runnables in the raw app. Returns a record of runnable keys to their configuration. Use list_backend_runnables + get_backend_runnable for large apps.'
	)
)

const getInlineScriptSchema = memo(() =>
	z.object({
		language: z.enum(['bun', 'python3']).describe('The language of the inline script'),
		content: z
			.string()
			.describe('The content of the inline script. Must have a main function as entrypoint.')
	})
)

const getSetBackendRunnableSchema = memo(() =>
	z.object({
		key: z
			.string()
			.describe(
				'The unique key/identifier for the backend runnable (used to call it from frontend as backend.<key>())'
			),
		name: z.string().describe('A short summary/description of what the runnable does'),
		type: z
			.enum(['script', 'flow', 'hubscript', 'inline'])
			.describe(
				'The type of runnable: "inline" for custom code, "script" for workspace script, "flow" for workspace flow, "hubscript" for hub script'
			),
		staticInputs: z
			.record(z.string(), z.any())
			.optional()
			.describe(
				'Static inputs that are not overridable by the frontend caller. Useful for workspace/hub scripts to pre-fill certain arguments.'
			),
		inlineScript: getInlineScriptSchema()
			.optional()
			.describe('Required when type is "inline". Contains the language and content of the script.'),
		path: z
			.string()
			.optional()
			.describe(
				'Required when type is "script", "flow", or "hubscript". The fully qualified path to the workspace script/flow or hub script.'
			)
	})
)
const getSetBackendRunnableToolDef = memo(() =>
	createToolDef(
		getSetBackendRunnableSchema(),
		'set_backend_runnable',
		'Create or update a backend runnable. Use type "inline" for custom code, or reference existing workspace/hub scripts/flows. Returns lint diagnostics (errors and warnings).',
		{ strict: false }
	)
)

const getDeleteBackendRunnableSchema = memo(() =>
	z.object({
		key: z.string().describe('The key of the backend runnable to delete')
	})
)
const getDeleteBackendRunnableToolDef = memo(() =>
	createToolDef(
		getDeleteBackendRunnableSchema(),
		'delete_backend_runnable',
		'Delete a backend runnable from the raw app'
	)
)

// ============= Lint Tool =============

const getLintSchema = memo(() => z.object({}))
const getLintToolDef = memo(() =>
	createToolDef(
		getLintSchema(),
		'lint',
		'Lint all frontend files and backend runnables. Returns errors and warnings grouped by frontend/backend. Use this to check for issues after making changes.'
	)
)

// ============= Combined Files Tool =============

const getGetFilesSchema = memo(() => z.object({}))
const getGetFilesToolDef = memo(() =>
	createToolDef(
		getGetFilesSchema(),
		'get_files',
		'Get all files in the raw app, including both frontend files and backend runnables as separate records.'
	)
)

// ============= Selected Context Tool =============

const getGetSelectedContextSchema = memo(() => z.object({}))
const getGetSelectedContextToolDef = memo(() =>
	createToolDef(
		getGetSelectedContextSchema(),
		'get_selected_context',
		'Get information about what is currently selected in the app editor. Returns the type of selection (frontend file or backend runnable) and the path/key of the selected item.'
	)
)

// ============= Workspace Runnables Search =============

const getListWorkspaceRunnablesSchema = memo(() =>
	z.object({
		query: z.string().describe('The search query to find workspace scripts and flows'),
		type: z
			.enum(['all', 'scripts', 'flows'])
			.describe(
				'Filter by type: "scripts" for scripts only, "flows" for flows only, "all" for both. Defaults to "all".'
			)
	})
)
const getListWorkspaceRunnablesToolDef = memo(() =>
	createToolDef(
		getListWorkspaceRunnablesSchema(),
		'list_workspace_runnables',
		'Search for workspace scripts and flows by query. Returns fully qualified paths that can be used in backend runnables.'
	)
)

class WorkspaceRunnablesSearch {
	private uf: uFuzzy
	private workspace: string | undefined = undefined
	private scripts: Script[] | undefined = undefined
	private flows: Flow[] | undefined = undefined

	constructor() {
		this.uf = new uFuzzy()
	}

	private async initScripts(workspace: string) {
		if (this.scripts === undefined || this.workspace !== workspace) {
			this.scripts = await ScriptService.listScripts({ workspace })
			this.workspace = workspace
		}
	}

	private async initFlows(workspace: string) {
		if (this.flows === undefined || this.workspace !== workspace) {
			this.flows = await FlowService.listFlows({ workspace })
			this.workspace = workspace
		}
	}

	async searchScripts(query: string, workspace: string) {
		await this.initScripts(workspace)
		const scripts = this.scripts
		if (!scripts) return []

		const results = this.uf.search(
			scripts.map((s) => (emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')')),
			query.trim()
		)
		return (
			results[2]?.map((id) => ({
				type: 'script' as const,
				path: scripts[id].path,
				summary: scripts[id].summary
			})) ?? []
		)
	}

	async searchFlows(query: string, workspace: string) {
		await this.initFlows(workspace)
		const flows = this.flows
		if (!flows) return []

		const results = this.uf.search(
			flows.map((f) => (emptyString(f.summary) ? f.path : f.summary + ' (' + f.path + ')')),
			query.trim()
		)
		return (
			results[2]?.map((id) => ({
				type: 'flow' as const,
				path: flows[id].path,
				summary: flows[id].summary
			})) ?? []
		)
	}

	async search(query: string, workspace: string, type: 'all' | 'scripts' | 'flows' = 'all') {
		const results: { type: 'script' | 'flow'; path: string; summary: string }[] = []

		if (type === 'all' || type === 'scripts') {
			results.push(...(await this.searchScripts(query, workspace)))
		}
		if (type === 'all' || type === 'flows') {
			results.push(...(await this.searchFlows(query, workspace)))
		}

		return results
	}
}

const workspaceRunnablesSearch = new WorkspaceRunnablesSearch()

// ============= Lint Result Formatting =============

function formatLintMessages(messages: Record<string, string[]>): string {
	let result = ''
	for (const [path, msgs] of Object.entries(messages)) {
		if (msgs.length > 0) {
			result += `\n**${path}**\n`
			for (const msg of msgs) {
				result += `- ${msg}\n`
			}
		}
	}
	return result
}

function formatLintResultResponse(message: string, lintResult: LintResult): string {
	let response = message
	const hasIssues = lintResult.errorCount > 0 || lintResult.warningCount > 0

	if (hasIssues) {
		response += '\n\n## Lint Diagnostics\n'

		if (lintResult.errorCount > 0) {
			response += `\n❌ **${lintResult.errorCount} error(s)** found that must be fixed.\n`

			const frontendErrors = formatLintMessages(lintResult.errors.frontend)
			if (frontendErrors) {
				response += `\n### Frontend Errors\n${frontendErrors}`
			}

			const backendErrors = formatLintMessages(lintResult.errors.backend)
			if (backendErrors) {
				response += `\n### Backend Errors\n${backendErrors}`
			}
		}

		if (lintResult.warningCount > 0) {
			response += `\n⚠️ **${lintResult.warningCount} warning(s)** found.\n`

			const frontendWarnings = formatLintMessages(lintResult.warnings.frontend)
			if (frontendWarnings) {
				response += `\n### Frontend Warnings\n${frontendWarnings}`
			}

			const backendWarnings = formatLintMessages(lintResult.warnings.backend)
			if (backendWarnings) {
				response += `\n### Backend Warnings\n${backendWarnings}`
			}
		}
	} else {
		response += '\n\n✅ No lint issues found.'
	}

	return response
}

// ============= Tools Array =============

export const getAppTools = memo((): Tool<AppAIChatHelpers>[] => [
	// Combined files tool
	{
		def: getGetFilesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting all files...' })
			const files = helpers.getFiles()
			const frontendCount = Object.keys(files.frontend).length
			const backendCount = Object.keys(files.backend).length
			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved ${frontendCount} frontend files and ${backendCount} backend runnables`
			})
			return JSON.stringify(files, null, 2)
		}
	},
	// Selected context tool
	{
		def: getGetSelectedContextToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting selected context...' })
			const context = helpers.getSelectedContext()
			const statusMsg =
				context.type === 'frontend'
					? `Frontend file selected: ${context.frontendPath}`
					: context.type === 'backend'
						? `Backend runnable selected: ${context.backendKey}`
						: 'No selection'
			toolCallbacks.setToolStatus(toolId, { content: statusMsg })
			return JSON.stringify(context, null, 2)
		}
	},
	// Frontend tools - list
	{
		def: getListFrontendFilesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Listing frontend files...' })
			const paths = helpers.listFrontendFiles()
			toolCallbacks.setToolStatus(toolId, { content: `Found ${paths.length} frontend files` })
			return JSON.stringify(paths, null, 2)
		}
	},
	{
		def: getGetFrontendFileToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getGetFrontendFileSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Getting frontend file '${parsedArgs.path}'...`
			})
			const content = helpers.getFrontendFile(parsedArgs.path)
			if (content === undefined) {
				const errorMsg = `Frontend file '${parsedArgs.path}' not found`
				toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
				return errorMsg
			}
			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved frontend file '${parsedArgs.path}'`
			})
			return content
		}
	},
	{
		def: getGetFrontendFilesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting frontend files...' })
			const files = helpers.getFrontendFiles()
			const fileCount = Object.keys(files).length
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved ${fileCount} frontend files` })
			return JSON.stringify(files, null, 2)
		}
	},
	{
		def: getSetFrontendFileToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getSetFrontendFileSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Setting frontend file '${parsedArgs.path}'...`
			})
			const lintResult = helpers.setFrontendFile(parsedArgs.path, parsedArgs.content)
			toolCallbacks.setToolStatus(toolId, {
				content: `Frontend file '${parsedArgs.path}' updated`,
				result: 'Success'
			})
			return formatLintResultResponse(
				`Frontend file '${parsedArgs.path}' has been set successfully.`,
				lintResult
			)
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting frontend file...' })
		},
		streamArguments: true,
		showDetails: true,
		showFade: true
	},
	{
		def: getDeleteFrontendFileToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getDeleteFrontendFileSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Deleting frontend file '${parsedArgs.path}'...`
			})
			helpers.deleteFrontendFile(parsedArgs.path)
			toolCallbacks.setToolStatus(toolId, { content: `Frontend file '${parsedArgs.path}' deleted` })
			return `Frontend file '${parsedArgs.path}' has been deleted successfully`
		}
	},
	// Backend tools - list
	{
		def: getListBackendRunnablesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Listing backend runnables...' })
			const list = helpers.listBackendRunnables()
			toolCallbacks.setToolStatus(toolId, { content: `Found ${list.length} backend runnables` })
			return JSON.stringify(list, null, 2)
		}
	},
	{
		def: getGetBackendRunnableToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getGetBackendRunnableSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Getting backend runnable '${parsedArgs.key}'...`
			})
			const runnable = helpers.getBackendRunnable(parsedArgs.key)
			if (runnable === undefined) {
				const errorMsg = `Backend runnable '${parsedArgs.key}' not found`
				toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
				return errorMsg
			}
			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved backend runnable '${parsedArgs.key}'`
			})
			return JSON.stringify(runnable, null, 2)
		}
	},
	{
		def: getGetBackendRunnablesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting backend runnables...' })
			const runnables = helpers.getBackendRunnables()
			const count = Object.keys(runnables).length
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved ${count} backend runnables` })
			return JSON.stringify(runnables, null, 2)
		}
	},
	{
		def: getSetBackendRunnableToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			// Convert null to undefined for optional fields (AI models sometimes send null instead of omitting fields)
			const cleanedArgs = {
				...args,
				staticInputs: args.staticInputs === null ? undefined : args.staticInputs,
				inlineScript: args.inlineScript === null ? undefined : args.inlineScript,
				path: args.path === null ? undefined : args.path
			}

			const parsedArgs = getSetBackendRunnableSchema().parse(cleanedArgs)
			toolCallbacks.setToolStatus(toolId, {
				content: `Setting backend runnable '${parsedArgs.key}'...`
			})

			const runnable: BackendRunnable = {
				name: parsedArgs.name,
				type: parsedArgs.type,
				...(parsedArgs.staticInputs && { staticInputs: parsedArgs.staticInputs }),
				...(parsedArgs.inlineScript && { inlineScript: parsedArgs.inlineScript }),
				...(parsedArgs.path && { path: parsedArgs.path })
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Backend runnable '${parsedArgs.key}' updated, waiting for analysis...`
			})
			const lintResult = await helpers.setBackendRunnable(parsedArgs.key, runnable)
			toolCallbacks.setToolStatus(toolId, {
				content: `Backend runnable '${parsedArgs.key}' analyzed`,
				result: 'Success'
			})
			return formatLintResultResponse(
				`Backend runnable '${parsedArgs.key}' has been set successfully.`,
				lintResult
			)
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting backend runnable...' })
		},
		streamArguments: true,
		showDetails: true,
		showFade: true
	},
	{
		def: getDeleteBackendRunnableToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getDeleteBackendRunnableSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Deleting backend runnable '${parsedArgs.key}'...`
			})
			helpers.deleteBackendRunnable(parsedArgs.key)
			toolCallbacks.setToolStatus(toolId, {
				content: `Backend runnable '${parsedArgs.key}' deleted`
			})
			return `Backend runnable '${parsedArgs.key}' has been deleted successfully`
		}
	},
	// Lint tool
	{
		def: getLintToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Linting app...' })
			const lintResult = helpers.lint()
			const status =
				lintResult.errorCount > 0
					? `Found ${lintResult.errorCount} error(s)`
					: lintResult.warningCount > 0
						? `Found ${lintResult.warningCount} warning(s)`
						: 'No issues found'
			toolCallbacks.setToolStatus(toolId, { content: status })
			return formatLintResultResponse('Lint completed.', lintResult)
		}
	},
	// Search tools
	{
		def: getListWorkspaceRunnablesToolDef(),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = getListWorkspaceRunnablesSchema().parse(args)
			const type = parsedArgs.type ?? 'all'
			toolCallbacks.setToolStatus(toolId, {
				content: `Searching workspace ${type} for "${parsedArgs.query}"...`
			})

			const results = await workspaceRunnablesSearch.search(parsedArgs.query, workspace, type)

			toolCallbacks.setToolStatus(toolId, {
				content: `Found ${results.length} workspace runnables matching "${parsedArgs.query}"`
			})
			return JSON.stringify(results, null, 2)
		}
	},
	// Hub scripts search (reuse from shared)
	createSearchHubScriptsTool(false)
])

export function prepareAppSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits apps on the Windmill platform. Apps are defined as a collection of files that contains both the frontend and the backend.

## App Structure

### Frontend
- The frontend is bundled using esbuild with entrypoint \`index.tsx\`
- Frontend files are managed separately from backend runnables
- The \`wmill.d.ts\` file is generated automatically from the backend runnables shape

### Backend
Backend runnables can be of different types:
- **inline**: Custom code written directly in the app (TypeScript/Bun or Python)
- **script**: Reference to a workspace script by path
- **flow**: Reference to a workspace flow by path
- **hubscript**: Reference to a hub script by path

Frontend calls backend using \`await backend.<runnable_key>(args...)\`.

For inline scripts, the code must have a \`main\` function as its entrypoint.

## Available Tools

### File Management
- \`get_files()\`: Get both frontend files and backend runnables (use for small apps or full overview)
- \`list_frontend_files()\`: List all frontend file paths without content (efficient for large apps)
- \`get_frontend_file(path)\`: Get content of a specific frontend file
- \`get_frontend_files()\`: Get all frontend files with content (use list + get for large apps)
- \`set_frontend_file(path, content)\`: Create or update a frontend file. Returns lint diagnostics.
- \`delete_frontend_file(path)\`: Delete a frontend file
- \`list_backend_runnables()\`: List all backend runnable keys and names (efficient for large apps)
- \`get_backend_runnable(key)\`: Get full configuration of a specific backend runnable
- \`get_backend_runnables()\`: Get all backend runnables (use list + get for large apps)
- \`set_backend_runnable(key, name, type, ...)\`: Create or update a backend runnable. Returns lint diagnostics.
- \`delete_backend_runnable(key)\`: Delete a backend runnable

### Linting
- \`lint()\`: Lint all files. Returns errors/warnings grouped by frontend/backend. Use this to check for issues after making changes.

### Discovery
- \`list_workspace_runnables(query, type?)\`: Search workspace scripts and flows
- \`search_hub_scripts(query)\`: Search hub scripts

### Best Practices
For large apps with many files or runnables:
1. Use \`list_frontend_files()\` or \`list_backend_runnables()\` first to get an overview
2. Then use \`get_frontend_file(path)\` or \`get_backend_runnable(key)\` to inspect specific items
3. This approach is more efficient and avoids overwhelming the context with too much content

## Backend Runnable Configuration

When creating a backend runnable with \`set_backend_runnable\`:

1. **For inline scripts** (type: "inline"):
   \`\`\`
   {
     key: "myFunction",
     name: "Does something useful",
     type: "inline",
     inlineScript: {
       language: "bun",  // or "python3"
       content: "export async function main(arg1: string) { return result; }"
     }
   }
   \`\`\`

2. **For workspace scripts** (type: "script"):
   \`\`\`
   {
     key: "sendEmail",
     name: "Send email via SMTP",
     type: "script",
     path: "f/folder/send_email",
     staticInputs: { smtp_server: "mail.example.com" }  // optional pre-filled inputs
   }
   \`\`\`

3. **For workspace flows** (type: "flow"):
   \`\`\`
   {
     key: "processOrder",
     name: "Process customer order",
     type: "flow",
     path: "f/folder/process_order_flow"
   }
   \`\`\`

4. **For hub scripts** (type: "hubscript"):
   \`\`\`
   {
     key: "slackMessage",
     name: "Send Slack message",
     type: "hubscript",
     path: "hub/123/slack/send_message"
   }
   \`\`\`

## Instructions

Follow the user instructions carefully. When creating a new app:
1. First use \`get_files\` to see the current state (includes wmill.d.ts showing how to call backend functions)
2. Create frontend files using \`set_frontend_file\`. This returns lint diagnostics.
3. Create backend runnables using \`set_backend_runnable\`. This returns lint diagnostics.
4. Use \`lint()\` to check for errors at any time
5. Use \`list_workspace_runnables\` or \`search_hub_scripts\` to find existing scripts/flows to reuse
6. Always fix any lint errors before finishing

Windmill expects all backend runnable calls to use an object parameter structure. For example for:
\`\`\`typescript
export async function main(arg1: string, arg2: string, arg3: number, arg4: { field1: string, field2: number }) {
  ...
}
\`\`\`

You would call it like this:
\`\`\`typescript
await backend.myFunction({ arg1: 'value1', arg2: 'value2', arg3: 3, arg4: { field1: 'value1', field2: 2 } })
\`\`\`
If the runnable has no parameters, you can call it without an object:
\`\`\`typescript
await backend.myFunction()
\`\`\`

When you are using the windmill-client, do not forget that as id for variables or resources, those are path that are of the form \'u/<user>/<name>\' or \'f/<folder>/<name>\'.

Explain what you're doing as you work. Show file contents before setting them when making significant changes.
`

	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareAppUserMessage(
	instructions: string,
	files?: AppFiles,
	selectedContext?: SelectedContext
): ChatCompletionUserMessageParam {
	let content = ''

	if (selectedContext && selectedContext.type !== 'none') {
		content += `## SELECTED CONTEXT:\n`
		if (selectedContext.type === 'frontend') {
			content += `The user is currently viewing the frontend file: ${selectedContext.frontendPath}\n\n`
		} else if (selectedContext.type === 'backend') {
			content += `The user is currently viewing the backend runnable: ${selectedContext.backendKey}\n\n`
		}
	}

	if (files) {
		if (Object.keys(files.frontend).length > 0) {
			content += `## CURRENT FRONTEND FILES:\n`
			for (const [path, fileContent] of Object.entries(files.frontend)) {
				content += `\n### ${path}\n\`\`\`\n${fileContent}\n\`\`\`\n`
			}
			content += '\n'
		}

		if (Object.keys(files.backend).length > 0) {
			content += `## CURRENT BACKEND RUNNABLES:\n`
			content += '\`\`\`json\n' + JSON.stringify(files.backend, null, 2) + '\n\`\`\`\n\n'
		}
	}

	content += `## INSTRUCTIONS:\n${instructions}`

	return {
		role: 'user',
		content
	}
}
