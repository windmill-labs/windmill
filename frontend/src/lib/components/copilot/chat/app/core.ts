import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import {
	createSearchHubScriptsTool,
	createToolDef,
	createSearchWorkspaceTool,
	createGetRunnableDetailsTool,
	type Tool
} from '../shared'
import { aiChatManager } from '../AIChatManager.svelte'
import type {
	ContextElement,
	AppFrontendFileElement,
	AppBackendRunnableElement,
	AppCodeSelectionElement,
	AppDatatableElement
} from '../context'

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

/** Information about an element selected via the inspector tool */
export interface InspectorElementInfo {
	/** CSS selector path to the element */
	path: string
	/** Tag name of the element (e.g., 'div', 'button') */
	tagName: string
	/** Element's id attribute, if any */
	id?: string
	/** Element's class names, if any */
	className?: string
	/** Bounding rectangle of the element */
	rect: { top: number; left: number; width: number; height: number }
	/** Outer HTML of the element (may be truncated) */
	html: string
	/** Text content of the element (may be truncated) */
	textContent?: string
	/** Computed styles of the element */
	styles: Record<string, string>
}

/** Context about the currently selected file or runnable in the app editor */
export interface SelectedContext {
	/** Type of selection: 'frontend' for frontend files, 'backend' for backend runnables, or 'none' if nothing is selected */
	type: 'frontend' | 'backend' | 'none'
	/** The path of the selected frontend file (when type is 'frontend') */
	frontendPath?: string
	/** The content of the selected frontend file */
	frontendContent?: string
	/** The key of the selected backend runnable (when type is 'backend') */
	backendKey?: string
	/** The configuration of the selected backend runnable */
	backendRunnable?: BackendRunnable
	/** Inspector-selected element info (when user has used the inspector tool) */
	inspectorElement?: InspectorElementInfo
	/** Whether the file/runnable selection is excluded from being sent to the AI prompt */
	selectionExcluded?: boolean
	/** Function to toggle whether the selection is excluded from the prompt */
	toggleSelectionExcluded?: () => void
	/** Function to clear the inspector selection */
	clearInspector?: () => void
	/** Function to clear the runnable selection (go back to frontend view) */
	clearRunnable?: () => void
	/** Code selection from the editor (either frontend or backend) */
	codeSelection?: AppCodeSelectionElement
	/** Function to clear the code selection */
	clearCodeSelection?: () => void
}

/**
 * Datatable schema matching the backend's DataTableSchema type.
 * Hierarchical structure: schema_name -> table_name -> column_name -> compact_type
 * Compact type format: 'type[?][=default]' where ? means nullable (e.g. 'int4', 'text?', 'int4?=0')
 */
export interface DataTableSchema {
	datatable_name: string
	schemas: Record<string, Record<string, Record<string, string>>>
	error?: string
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
	// Data table operations
	/** Get all datatables configured in the app with their schemas */
	getDatatables: () => Promise<DataTableSchema[]>
	/** Get unique datatable names configured in the app (for UI policy selector) */
	getAvailableDatatableNames: () => string[]
	/** Execute a SQL query on a datatable. Optionally specify newTable to register a newly created table. */
	execDatatableSql: (
		datatableName: string,
		sql: string,
		newTable?: { schema: string; name: string }
	) => Promise<{ success: boolean; result?: Record<string, any>[]; error?: string }>
	/** Add a table to the app's whitelisted tables (called when user selects a table via @) */
	addTableToWhitelist: (datatableName: string, schemaName: string, tableName: string) => void
}

// ============= Utility =============

/** Maximum characters per file in get_files tool */
const BATCH_FILE_SIZE_LIMIT = 2500

/** Memoize a factory function - the factory is only called once, on first access */
const memo = <T>(factory: () => T): (() => T) => {
	let cached: T | undefined
	return () => (cached ??= factory())
}

// ============= Frontend File Tools =============

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
		'Get an overview of all files in the app. Content may be truncated for large apps - use get_frontend_file or get_backend_runnable for full content of specific files.'
	)
)

// ============= Data Table Tools =============

const getGetDatatablesSchema = memo(() => z.object({}))
const getGetDatatablesToolDef = memo(() =>
	createToolDef(
		getGetDatatablesSchema(),
		'get_datatables',
		'Get all datatables configured in this app with their full schemas. Returns datatable names, tables, and column definitions. Use this to understand the data layer available to the app.'
	)
)

const getExecDatatableSqlSchema = memo(() =>
	z.object({
		datatable_name: z
			.string()
			.describe(
				'The name of the datatable to query (e.g., "main"). Must be one of the datatables configured in the app.'
			),
		sql: z
			.string()
			.describe(
				'The SQL query to execute. Supports SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, etc. For SELECT queries, results are returned as an array of objects.'
			),
		new_table: z
			.object({
				schema: z.string().describe('The schema name where the table was created (e.g., "public")'),
				name: z.string().describe('The name of the newly created table')
			})
			.optional()
			.describe(
				'When executing a CREATE TABLE statement, provide this to register the new table in the app so it can be queried and its schema retrieved later.'
			)
	})
)
const getExecDatatableSqlToolDef = memo(() =>
	createToolDef(
		getExecDatatableSqlSchema(),
		'exec_datatable_sql',
		'Execute a SQL query on a datatable. Use this to explore data, test queries, create tables, or make changes. When creating a new table, pass new_table to register it in the app for future use.',
		{ strict: false }
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
	// Combined files tool (with per-file truncation for large apps)
	{
		def: getGetFilesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting all files...' })
			const files = helpers.getFiles()
			const frontendCount = Object.keys(files.frontend).length
			const backendCount = Object.keys(files.backend).length

			// Truncate each file individually to BATCH_FILE_SIZE_LIMIT
			let anyTruncated = false
			const truncatedFiles: AppFiles = {
				frontend: {},
				backend: {}
			}

			for (const [path, content] of Object.entries(files.frontend)) {
				if (content.length > BATCH_FILE_SIZE_LIMIT) {
					truncatedFiles.frontend[path] =
						content.slice(0, BATCH_FILE_SIZE_LIMIT) + '\n... [TRUNCATED]'
					anyTruncated = true
				} else {
					truncatedFiles.frontend[path] = content
				}
			}

			for (const [key, runnable] of Object.entries(files.backend)) {
				const runnableCopy = { ...runnable }
				if (runnableCopy.inlineScript?.content) {
					const content = runnableCopy.inlineScript.content
					if (content.length > BATCH_FILE_SIZE_LIMIT) {
						runnableCopy.inlineScript = {
							...runnableCopy.inlineScript,
							content: content.slice(0, BATCH_FILE_SIZE_LIMIT) + '\n... [TRUNCATED]'
						}
						anyTruncated = true
					}
				}
				truncatedFiles.backend[key] = runnableCopy
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Retrieved ${frontendCount} frontend files and ${backendCount} backend runnables${anyTruncated ? ' (some truncated)' : ''}`
			})

			let result = JSON.stringify(truncatedFiles, null, 2)
			if (anyTruncated) {
				result +=
					'\n\nNote: Some file contents were truncated. Use get_frontend_file(path) or get_backend_runnable(key) to get full content.'
			}

			return result
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
	// Frontend tools
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
	// Backend tools
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
				content: `Backend runnable '${parsedArgs.key}' set successfully.`,
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
	createSearchWorkspaceTool(),
	createGetRunnableDetailsTool(),
	// Hub scripts search (reuse from shared)
	createSearchHubScriptsTool(false),
	// Data table tools
	{
		def: getGetDatatablesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting datatables...' })
			try {
				const datatables = await helpers.getDatatables()
				if (datatables.length === 0) {
					toolCallbacks.setToolStatus(toolId, { content: 'No datatables configured' })
					return 'No datatables are configured in this app. Use the Data panel in the sidebar to add datatable references.'
				}
				const totalTables = datatables.reduce(
					(acc, dt) =>
						acc +
						Object.values(dt.schemas).reduce(
							(schemaAcc, tables) => schemaAcc + Object.keys(tables).length,
							0
						),
					0
				)
				toolCallbacks.setToolStatus(toolId, {
					content: `Found ${datatables.length} datatable(s) with ${totalTables} table(s)`
				})
				return JSON.stringify(datatables, null, 2)
			} catch (e) {
				const errorMsg = `Error getting datatables: ${e instanceof Error ? e.message : String(e)}`
				toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
				return errorMsg
			}
		}
	},
	{
		def: getExecDatatableSqlToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getExecDatatableSqlSchema().parse(args)

			// Enforce datatable creation policy when new_table is specified
			if (parsedArgs.new_table) {
				const policy = aiChatManager.datatableCreationPolicy
				if (!policy.enabled) {
					const errorMsg =
						'Table creation is not allowed. The user has disabled the "New tables" option.'
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return JSON.stringify({ success: false, error: errorMsg })
				}
				if (policy.datatable && policy.datatable !== parsedArgs.datatable_name) {
					const errorMsg = `Table creation is only allowed on datatable "${policy.datatable}", but you tried to create on "${parsedArgs.datatable_name}".`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return JSON.stringify({ success: false, error: errorMsg })
				}
				if (policy.schema && policy.schema !== parsedArgs.new_table.schema) {
					const errorMsg = `Table creation is only allowed in schema "${policy.schema}", but you tried to create in "${parsedArgs.new_table.schema}".`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return JSON.stringify({ success: false, error: errorMsg })
				}
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Executing SQL on "${parsedArgs.datatable_name}"...`
			})
			try {
				const result = await helpers.execDatatableSql(
					parsedArgs.datatable_name,
					parsedArgs.sql,
					parsedArgs.new_table
				)
				if (result.success) {
					let successMessage = 'Query executed successfully'
					if (parsedArgs.new_table) {
						successMessage = `Table "${parsedArgs.new_table.schema}.${parsedArgs.new_table.name}" created and registered`
					}
					if (result.result) {
						const rowCount = result.result.length
						toolCallbacks.setToolStatus(toolId, {
							content: `Query returned ${rowCount} row(s)`
						})
						// Truncate large results
						const MAX_ROWS = 100
						if (rowCount > MAX_ROWS) {
							return JSON.stringify(
								{
									success: true,
									rowCount,
									result: result.result.slice(0, MAX_ROWS),
									note: `Showing first ${MAX_ROWS} of ${rowCount} rows`
								},
								null,
								2
							)
						}
						return JSON.stringify({ success: true, rowCount, result: result.result }, null, 2)
					}
					toolCallbacks.setToolStatus(toolId, { content: successMessage })
					return JSON.stringify({ success: true, message: successMessage })
				} else {
					const errorMsg = result.error || 'Unknown error'
					toolCallbacks.setToolStatus(toolId, { content: `Error: ${errorMsg}`, error: errorMsg })
					return JSON.stringify({ success: false, error: errorMsg })
				}
			} catch (e) {
				const errorMsg = e instanceof Error ? e.message : String(e)
				toolCallbacks.setToolStatus(toolId, { content: `Error: ${errorMsg}`, error: errorMsg })
				return JSON.stringify({ success: false, error: errorMsg })
			}
		}
	}
])

export function prepareAppSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	// Get policy early so we can use it in the template
	const policy = aiChatManager.datatableCreationPolicy
	const datatableName = policy.datatable ?? 'main'
	const schemaPrefix = policy.schema ? `${policy.schema}.` : ''
	// Use wmill.datatable() for 'main' (default), otherwise wmill.datatable('name')
	const datatableCall =
		datatableName === 'main' ? 'wmill.datatable()' : `wmill.datatable('${datatableName}')`

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
- \`get_files()\`: Get an overview of all files (content may be truncated for large files)
- \`get_frontend_file(path)\`: Get full content of a specific frontend file
- \`set_frontend_file(path, content)\`: Create or update a frontend file. Returns lint diagnostics.
- \`delete_frontend_file(path)\`: Delete a frontend file
- \`get_backend_runnable(key)\`: Get full configuration of a specific backend runnable
- \`set_backend_runnable(key, name, type, ...)\`: Create or update a backend runnable. Returns lint diagnostics.
- \`delete_backend_runnable(key)\`: Delete a backend runnable

### Linting
- \`lint()\`: Lint all files. Returns errors/warnings grouped by frontend/backend. Use this to check for issues after making changes.

### Discovery
- \`search_workspace(query, type)\`: Search workspace scripts and flows
- \`get_runnable_details(path, type)\`: Get details (summary, description, schema, content) of a specific script or flow
- \`search_hub_scripts(query)\`: Search hub scripts

### Data Tables
- \`get_datatables()\`: Get all datatables configured in the app with their schemas (tables, columns, types)
- \`exec_datatable_sql(datatable_name, sql, new_table?)\`: Execute SQL query on a datatable. Use for data exploration or modifications. When creating a new table, pass \`new_table: { schema, name }\` to register it in the app.

## Data Storage with Data Tables

**When the app needs to store or persist data, you MUST use datatables.** Datatables provide a managed PostgreSQL database that integrates seamlessly with Windmill apps, with near-zero setup and workspace-scoped access.

### Key Principles

1. **Always check existing tables first**: Use \`get_datatables()\` to see what tables are already available. If a suitable table exists, **always reuse it** rather than creating a new one.

2. **CRITICAL: Create tables ONLY via exec_datatable_sql tool**: When you need to create a new table, you MUST use the \`exec_datatable_sql\` tool with the \`new_table\` parameter. **NEVER** create tables inside backend runnables using SQL queries - this will not register the table properly and it won't be available for future use.
   \`\`\`
   exec_datatable_sql({
     datatable_name: "main",
     sql: "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE, created_at TIMESTAMP DEFAULT NOW())",
     new_table: { schema: "public", name: "users" }
   })
   \`\`\`

3. **Use schemas to organize data**: Use PostgreSQL schemas to organize tables logically. Reference schemas with \`schema.table\` syntax.

4. **Use datatables for**:
   - User data, settings, preferences
   - Application state that needs to persist
   - Lists, records, logs, history
   - Any data the app needs to store and retrieve

### Accessing Data Tables from Backend Runnables

Backend runnables should only perform **data operations** (SELECT, INSERT, UPDATE, DELETE) on **existing tables**. Never use CREATE TABLE, DROP TABLE, or ALTER TABLE inside runnables.

**TypeScript (Bun)**:
\`\`\`typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = ${datatableCall};

  // Safe string interpolation (parameterized query)
  const user = await sql\`SELECT * FROM ${schemaPrefix}users WHERE id = \${user_id}\`.fetchOne();
  return user;
}
\`\`\`

**Python**:
\`\`\`python
import wmill

def main(user_id: str):
    db = ${datatableCall}

    # Use positional arguments ($1, $2, etc.)
    user = db.query('SELECT * FROM ${schemaPrefix}users WHERE id = $1', user_id).fetch_one()
    return user
\`\`\`

### Common Operations (for use in backend runnables)

- **Fetch all**: \`sql\`SELECT * FROM ${schemaPrefix}table\`.fetch()\` or \`db.query('SELECT * FROM ${schemaPrefix}table').fetch()\`
- **Fetch one**: \`.fetchOne()\` or \`.fetch_one()\`
- **Insert**: \`sql\`INSERT INTO ${schemaPrefix}table (col) VALUES (\${value})\`\`
- **Update**: \`sql\`UPDATE ${schemaPrefix}table SET col = \${value} WHERE id = \${id}\`\`
- **Delete**: \`sql\`DELETE FROM ${schemaPrefix}table WHERE id = \${id}\`\`

### Schema Modifications (DDL) - Use exec_datatable_sql tool ONLY

For any schema changes (CREATE TABLE, DROP TABLE, ALTER TABLE, CREATE INDEX, etc.), you MUST use the \`exec_datatable_sql\` tool directly. This ensures tables are properly registered in the app.

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

## Instructions

1. Start with \`get_files()\` to get an overview of all frontend files and backend runnables (content may be truncated for large files)
2. Use \`get_frontend_file(path)\` or \`get_backend_runnable(key)\` to get full content of specific files when needed
3. Make changes using \`set_frontend_file\` and \`set_backend_runnable\`. These return lint diagnostics.
4. Use \`lint()\` at the end to check for and fix any remaining errors

When creating a new app, use \`search_workspace\` or \`search_hub_scripts\` to find existing scripts/flows to reuse.

`

	// Add datatable creation policy context
	if (policy.enabled && policy.datatable) {
		content += `## Datatable Creation Policy

**Table creation is ENABLED.** You can create new tables using \`exec_datatable_sql\` with the \`new_table\` parameter.
- **Default datatable**: ${datatableName}${policy.schema ? `\n- **Default schema**: ${policy.schema}` : ''}

When creating new tables, you MUST use the default datatable${policy.schema ? ` and schema` : ''} specified above. Do not create tables in other datatables or schemas.
${policy.schema ? `\n**IMPORTANT**: Always use the schema prefix \`${schemaPrefix}\` in your SQL queries when creating or referencing tables. For example: \`CREATE TABLE ${schemaPrefix}my_table (...)\` and \`SELECT * FROM ${schemaPrefix}my_table\`. Never create tables without the schema prefix as they would go to the public schema instead.` : ''}

`
	} else {
		content += `## Datatable Creation Policy

**Table creation is DISABLED.** You must NOT create new datatable tables. If you need to create a table to complete the task, inform the user that table creation is disabled and ask them to enable it in the Data panel settings.

`
	}

	if (customPrompt?.trim()) {
		content = `${content}\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

/** Maximum characters for file content in context */
const MAX_CONTEXT_CONTENT_LENGTH = 3000

export function prepareAppUserMessage(
	instructions: string,
	selectedContext?: SelectedContext,
	additionalContext?: ContextElement[]
): ChatCompletionUserMessageParam {
	let content = ''

	// Check if we have any context to add
	const hasSelectedContext =
		selectedContext && (selectedContext.type !== 'none' || selectedContext.inspectorElement)
	const hasAdditionalContext = additionalContext && additionalContext.length > 0

	if (hasSelectedContext || hasAdditionalContext) {
		content += `## SELECTED CONTEXT:\n`

		// Add frontend file context with content (unless excluded)
		if (
			selectedContext &&
			selectedContext.type === 'frontend' &&
			selectedContext.frontendPath &&
			!selectedContext.selectionExcluded
		) {
			content += `The user is currently viewing the frontend file: **${selectedContext.frontendPath}**\n`
			if (selectedContext.frontendContent) {
				const truncatedContent =
					selectedContext.frontendContent.length > MAX_CONTEXT_CONTENT_LENGTH
						? selectedContext.frontendContent.slice(0, MAX_CONTEXT_CONTENT_LENGTH) +
							'\n... [TRUNCATED]'
						: selectedContext.frontendContent
				content += `\n\`\`\`\n${truncatedContent}\n\`\`\`\n`
			}
		}

		// Add backend runnable context with content (unless excluded)
		if (
			selectedContext &&
			selectedContext.type === 'backend' &&
			selectedContext.backendKey &&
			!selectedContext.selectionExcluded
		) {
			content += `The user is currently viewing the backend runnable: **${selectedContext.backendKey}**\n`
			if (selectedContext.backendRunnable) {
				const runnable = selectedContext.backendRunnable
				content += `- **Name**: ${runnable.name}\n`
				content += `- **Type**: ${runnable.type}\n`
				if (runnable.path) {
					content += `- **Path**: ${runnable.path}\n`
				}
				if (runnable.inlineScript) {
					const truncatedCode =
						runnable.inlineScript.content.length > MAX_CONTEXT_CONTENT_LENGTH
							? runnable.inlineScript.content.slice(0, MAX_CONTEXT_CONTENT_LENGTH) +
								'\n... [TRUNCATED]'
							: runnable.inlineScript.content
					content += `- **Language**: ${runnable.inlineScript.language}\n`
					content += `- **Code**:\n\`\`\`${runnable.inlineScript.language === 'bun' ? 'typescript' : 'python'}\n${truncatedCode}\n\`\`\`\n`
				}
				if (runnable.staticInputs && Object.keys(runnable.staticInputs).length > 0) {
					content += `- **Static inputs**: ${JSON.stringify(runnable.staticInputs)}\n`
				}
			}
		}

		// Add inspector element context if available
		if (selectedContext?.inspectorElement) {
			const el = selectedContext.inspectorElement
			content += `\nThe user has selected an element in the app preview using the inspector tool:\n`
			content += `- **Element**: ${el.tagName}${el.id ? `#${el.id}` : ''}${el.className ? `.${el.className.split(' ').join('.')}` : ''}\n`
			content += `- **Selector path**: ${el.path}\n`
			content += `- **Dimensions**: ${Math.round(el.rect.width)}x${Math.round(el.rect.height)} at (${Math.round(el.rect.left)}, ${Math.round(el.rect.top)})\n`
			if (el.textContent) {
				const truncatedText =
					el.textContent.length > 100 ? el.textContent.slice(0, 100) + '...' : el.textContent
				content += `- **Text content**: "${truncatedText}"\n`
			}
			// Include HTML (truncated) for more context
			const truncatedHtml = el.html.length > 500 ? el.html.slice(0, 500) + '...' : el.html
			content += `- **HTML**:\n\`\`\`html\n${truncatedHtml}\n\`\`\`\n`
		}

		// Add code selection context if available
		if (selectedContext?.codeSelection) {
			const selection = selectedContext.codeSelection
			content += `\n### CODE SELECTION:\n`
			content += `The user has selected code in the ${selection.sourceType} editor:\n`
			content += `- **File/Source**: ${selection.source}\n`
			content += `- **Lines**: ${selection.startLine}-${selection.endLine}\n`
			const truncatedCode =
				selection.content.length > MAX_CONTEXT_CONTENT_LENGTH
					? selection.content.slice(0, MAX_CONTEXT_CONTENT_LENGTH) + '\n... [TRUNCATED]'
					: selection.content
			content += `\`\`\`\n${truncatedCode}\n\`\`\`\n`
		}

		// Add additional context from @ mentions
		if (additionalContext && additionalContext.length > 0) {
			content += `\n### ADDITIONAL CONTEXT (mentioned by user):\n`

			for (const ctx of additionalContext) {
				if (ctx.type === 'app_frontend_file') {
					const fileCtx = ctx as AppFrontendFileElement
					content += `\n**Frontend File: ${fileCtx.path}**\n`
					const truncatedContent =
						fileCtx.content.length > MAX_CONTEXT_CONTENT_LENGTH
							? fileCtx.content.slice(0, MAX_CONTEXT_CONTENT_LENGTH) + '\n... [TRUNCATED]'
							: fileCtx.content
					content += `\`\`\`\n${truncatedContent}\n\`\`\`\n`
				} else if (ctx.type === 'app_backend_runnable') {
					const runnableCtx = ctx as AppBackendRunnableElement
					const runnable = runnableCtx.runnable
					content += `\n**Backend Runnable: ${runnableCtx.key}**\n`
					content += `- **Name**: ${runnable.name}\n`
					content += `- **Type**: ${runnable.type}\n`
					if (runnable.path) {
						content += `- **Path**: ${runnable.path}\n`
					}
					if (runnable.inlineScript) {
						const truncatedCode =
							runnable.inlineScript.content.length > MAX_CONTEXT_CONTENT_LENGTH
								? runnable.inlineScript.content.slice(0, MAX_CONTEXT_CONTENT_LENGTH) +
									'\n... [TRUNCATED]'
								: runnable.inlineScript.content
						content += `- **Language**: ${runnable.inlineScript.language}\n`
						content += `- **Code**:\n\`\`\`${runnable.inlineScript.language === 'bun' ? 'typescript' : 'python'}\n${truncatedCode}\n\`\`\`\n`
					}
					if (runnable.staticInputs && Object.keys(runnable.staticInputs).length > 0) {
						content += `- **Static inputs**: ${JSON.stringify(runnable.staticInputs)}\n`
					}
				} else if (ctx.type === 'app_datatable') {
					const datatableCtx = ctx as AppDatatableElement
					const tableRef =
						datatableCtx.schemaName === 'public'
							? `${datatableCtx.datatableName}/${datatableCtx.tableName}`
							: `${datatableCtx.datatableName}/${datatableCtx.schemaName}:${datatableCtx.tableName}`
					content += `\n**Table: ${tableRef}**\n`
					content += `- **Datatable**: ${datatableCtx.datatableName}\n`
					content += `- **Schema**: ${datatableCtx.schemaName}\n`
					content += `- **Table**: ${datatableCtx.tableName}\n`
					// Format columns as column_name: type
					const columnsStr = JSON.stringify(datatableCtx.columns, null, 2)
					const truncatedColumns =
						columnsStr.length > MAX_CONTEXT_CONTENT_LENGTH
							? columnsStr.slice(0, MAX_CONTEXT_CONTENT_LENGTH) + '\n... [TRUNCATED]'
							: columnsStr
					content += `- **Columns** (column_name -> type):\n\`\`\`json\n${truncatedColumns}\n\`\`\`\n`
				}
			}
		}

		content += '\n'
	}

	content += `## INSTRUCTIONS:\n${instructions}`

	return {
		role: 'user',
		content
	}
}
