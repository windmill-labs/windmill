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
import {
	formatAppDatatableContextTitle,
	type ContextElement,
	type AppCodeSelectionElement,
	type AppDatatableElement
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

/** App editor context that is implicitly attached to app-mode AI messages. */
export interface SelectedContext {
	/** Inspector-selected element info (when user has used the inspector tool) */
	inspectorElement?: InspectorElementInfo
	/** Function to clear the inspector selection */
	clearInspector?: () => void
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

/** Memoize a factory function - the factory is only called once, on first access */
const memo = <T>(factory: () => T): (() => T) => {
	let cached: T | undefined
	return () => (cached ??= factory())
}

function countExactMatches(content: string, search: string): number {
	if (search.length === 0) {
		return 0
	}

	let count = 0
	let index = 0

	while ((index = content.indexOf(search, index)) !== -1) {
		count += 1
		index += search.length
	}

	return count
}

function replaceFirstExactMatch(content: string, search: string, replace: string): string {
	const index = content.indexOf(search)
	if (index === -1) {
		return content
	}

	return content.slice(0, index) + replace + content.slice(index + search.length)
}

type AppPatchTarget =
	| { type: 'frontend'; path: string }
	| { type: 'backend'; path: string; key: string; extension: 'ts' | 'py' }

function resolveAppPatchTarget(rawPath: string): AppPatchTarget {
	const trimmedPath = rawPath.trim()
	const backendMatch = trimmedPath.match(/^backend\/([^/]+)\/main\.(ts|py)$/)
	if (backendMatch) {
		return {
			type: 'backend',
			path: trimmedPath,
			key: backendMatch[1],
			extension: backendMatch[2] as 'ts' | 'py'
		}
	}

	return {
		type: 'frontend',
		path: trimmedPath.startsWith('/') ? trimmedPath : `/${trimmedPath}`
	}
}

function getBackendInlineScriptExtension(runnable: BackendRunnable): 'ts' | 'py' {
	return runnable.inlineScript?.language === 'python3' ? 'py' : 'ts'
}

function getFileKind(path: string): string {
	const filename = path.split('/').pop() ?? path
	const extension = filename.includes('.') ? filename.split('.').pop() : undefined
	return extension || 'unknown'
}

function getStaticInputKeys(runnable: BackendRunnable): string[] | undefined {
	const keys = runnable.staticInputs ? Object.keys(runnable.staticInputs) : []
	return keys.length > 0 ? keys : undefined
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

const getPatchFileSchema = memo(() =>
	z.object({
		path: z
			.string()
			.describe(
				'Path of the file to patch. Use frontend paths like /index.tsx or inline backend paths like backend/listRecipes/main.ts.'
			),
		old_string: z.string().min(1).describe('Exact text to find in the current file content'),
		new_string: z.string().describe('Replacement text'),
		replace_all: z
			.boolean()
			.optional()
			.default(false)
			.describe(
				'When true, replace every exact match. When false, old_string must match exactly once.'
			)
	})
)
const getPatchFileToolDef = memo(() =>
	createToolDef(
		getPatchFileSchema(),
		'patch_file',
		'Make a quick exact text edit in an existing frontend file or inline backend file. Prefer this for small localized changes instead of rewriting the whole file.'
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

// ============= File Listing Tool =============

interface AppFrontendFileMetadata {
	path: string
	size: number
	kind: string
}

interface AppBackendRunnableMetadata {
	key: string
	name: string
	type: BackendRunnableType
	path?: string
	language?: InlineScript['language']
	contentSize?: number
	staticInputKeys?: string[]
}

interface AppFilesMetadata {
	frontend: AppFrontendFileMetadata[]
	backend: AppBackendRunnableMetadata[]
}

const getListFilesSchema = memo(() => z.object({}))
const getListFilesToolDef = memo(() =>
	createToolDef(
		getListFilesSchema(),
		'list_files',
		'List lightweight metadata for frontend files and backend runnables in the app. Does not include file or runnable content. Use get_frontend_file(path) or get_backend_runnable(key) to inspect specific content.'
	)
)

// ============= Data Table Tools =============

interface AppDatatableMetadata {
	datatable_name: string
	schemas: Record<string, string[]>
	tableCount: number
	error?: string
}

function summarizeDatatables(datatables: DataTableSchema[]): AppDatatableMetadata[] {
	return datatables.map((datatable) => {
		const schemas: Record<string, string[]> = {}
		let tableCount = 0

		for (const [schemaName, tables] of Object.entries(datatable.schemas)) {
			const tableNames = Object.keys(tables)
			schemas[schemaName] = tableNames
			tableCount += tableNames.length
		}

		return {
			datatable_name: datatable.datatable_name,
			schemas,
			tableCount,
			...(datatable.error && { error: datatable.error })
		}
	})
}

const getListDatatablesSchema = memo(() => z.object({}))
const getListDatatablesToolDef = memo(() =>
	createToolDef(
		getListDatatablesSchema(),
		'list_datatables',
		'List datatables configured in the app with schema and table names only. Does not include column definitions. Use this directly for table-list or available-tables summaries. Only call get_datatable_table_schema when column names/types are required.'
	)
)

const getGetDatatableTableSchemaSchema = memo(() =>
	z.object({
		datatable_name: z.string().describe('The datatable name to inspect, e.g. "main".'),
		schema_name: z.string().describe('The schema name, e.g. "public".'),
		table_name: z.string().describe('The table name to inspect.')
	})
)
const getGetDatatableTableSchemaToolDef = memo(() =>
	createToolDef(
		getGetDatatableTableSchemaSchema(),
		'get_datatable_table_schema',
		'Get column definitions for one datatable table. Do not call this for row counts or table-list summaries; list_datatables is enough for those.'
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
	// Lightweight file/runnable metadata tool (no source contents)
	{
		def: getListFilesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Listing files...' })
			const files = helpers.getFiles()
			const metadata: AppFilesMetadata = {
				frontend: Object.entries(files.frontend).map(([path, content]) => ({
					path,
					size: content.length,
					kind: getFileKind(path)
				})),
				backend: Object.entries(files.backend).map(([key, runnable]) => {
					const staticInputKeys = getStaticInputKeys(runnable)
					return {
						key,
						name: runnable.name,
						type: runnable.type,
						...(runnable.path && { path: runnable.path }),
						...(runnable.inlineScript && {
							language: runnable.inlineScript.language,
							contentSize: runnable.inlineScript.content.length
						}),
						...(staticInputKeys && { staticInputKeys })
					}
				})
			}

			toolCallbacks.setToolStatus(toolId, {
				content: `Listed ${metadata.frontend.length} frontend files and ${metadata.backend.length} backend runnables`
			})

			return JSON.stringify(metadata, null, 2)
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
		def: getPatchFileToolDef(),
		streamArguments: true,
		showDetails: true,
		showFade: true,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getPatchFileSchema().parse(args)
			const { old_string: oldString, new_string: newString, replace_all: replaceAll } = parsedArgs
			const target = resolveAppPatchTarget(parsedArgs.path)
			let currentContent = ''
			let backendRunnable: BackendRunnable | undefined

			if (target.type === 'frontend') {
				if (target.path === '/wmill.d.ts') {
					throw new Error("'/wmill.d.ts' is generated automatically. Edit backend runnables instead.")
				}

				const frontendContent = helpers.getFrontendFile(target.path)
				if (frontendContent === undefined) {
					throw new Error(`Frontend file '${target.path}' not found.`)
				}
				currentContent = frontendContent
			} else {
				backendRunnable = helpers.getBackendRunnable(target.key)
				if (!backendRunnable) {
					throw new Error(`Backend runnable '${target.key}' not found.`)
				}
				if (backendRunnable.type !== 'inline' || !backendRunnable.inlineScript) {
					throw new Error(
						`'${target.path}' points to backend runnable '${target.key}', but only inline runnables can be patched as files.`
					)
				}

				const expectedExtension = getBackendInlineScriptExtension(backendRunnable)
				if (target.extension !== expectedExtension) {
					throw new Error(
						`'${target.path}' does not match runnable '${target.key}' language. Use backend/${target.key}/main.${expectedExtension}.`
					)
				}

				currentContent = backendRunnable.inlineScript.content ?? ''
			}

			const matchCount = countExactMatches(currentContent, oldString)
			if (matchCount === 0) {
				throw new Error('old_string was not found in the current file content.')
			}
			if (!replaceAll && matchCount !== 1) {
				throw new Error(
					`old_string matched ${matchCount} locations. Make it more specific or set replace_all to true.`
				)
			}

			const updatedContent = replaceAll
				? currentContent.split(oldString).join(newString)
				: replaceFirstExactMatch(currentContent, oldString, newString)

			toolCallbacks.setToolStatus(toolId, {
				content: `Patching '${target.path}'...`
			})
			const lintResult =
				target.type === 'frontend'
					? helpers.setFrontendFile(target.path, updatedContent)
					: await helpers.setBackendRunnable(target.key, {
							...backendRunnable!,
							inlineScript: {
								...backendRunnable!.inlineScript!,
								content: updatedContent
							}
						})
			toolCallbacks.setToolStatus(toolId, {
				content: `Patched '${target.path}'`,
				result: 'Success'
			})
			return formatLintResultResponse(`Patched '${target.path}' successfully.`, lintResult)
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Patching file...' })
		}
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
		def: getListDatatablesToolDef(),
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Listing datatables...' })
			try {
				const datatables = await helpers.getDatatables()
				if (datatables.length === 0) {
					toolCallbacks.setToolStatus(toolId, { content: 'No datatables configured' })
					return 'No datatables are configured in this app. Use the Data panel in the sidebar to add datatable references.'
				}
				const metadata = summarizeDatatables(datatables)
				const totalTables = metadata.reduce((acc, datatable) => acc + datatable.tableCount, 0)
				toolCallbacks.setToolStatus(toolId, {
					content: `Listed ${metadata.length} datatable(s) with ${totalTables} table(s)`
				})
				return JSON.stringify(metadata, null, 2)
			} catch (e) {
				const errorMsg = `Error listing datatables: ${e instanceof Error ? e.message : String(e)}`
				toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
				return errorMsg
			}
		}
	},
	{
		def: getGetDatatableTableSchemaToolDef(),
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = getGetDatatableTableSchemaSchema().parse(args)
			toolCallbacks.setToolStatus(toolId, {
				content: `Getting schema for ${parsedArgs.datatable_name}.${parsedArgs.schema_name}.${parsedArgs.table_name}...`
			})
			try {
				const datatables = await helpers.getDatatables()
				const datatable = datatables.find(
					(candidate) => candidate.datatable_name === parsedArgs.datatable_name
				)
				if (!datatable) {
					const availableDatatables = datatables.map((candidate) => candidate.datatable_name)
					const errorMsg = `Datatable '${parsedArgs.datatable_name}' not found. Available datatables: ${availableDatatables.join(', ') || 'none'}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}

				const schema = datatable.schemas[parsedArgs.schema_name]
				if (!schema) {
					const availableSchemas = Object.keys(datatable.schemas)
					const errorMsg = `Schema '${parsedArgs.schema_name}' not found in datatable '${parsedArgs.datatable_name}'. Available schemas: ${availableSchemas.join(', ') || 'none'}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}

				const columns = schema[parsedArgs.table_name]
				if (!columns) {
					const availableTables = Object.keys(schema)
					const errorMsg = `Table '${parsedArgs.table_name}' not found in '${parsedArgs.datatable_name}.${parsedArgs.schema_name}'. Available tables: ${availableTables.join(', ') || 'none'}`
					toolCallbacks.setToolStatus(toolId, { content: errorMsg, error: errorMsg })
					return errorMsg
				}

				toolCallbacks.setToolStatus(toolId, {
					content: `Retrieved schema for ${parsedArgs.schema_name}.${parsedArgs.table_name}`
				})
				return JSON.stringify(
					{
						datatable_name: parsedArgs.datatable_name,
						schema_name: parsedArgs.schema_name,
						table_name: parsedArgs.table_name,
						columns
					},
					null,
					2
				)
			} catch (e) {
				const errorMsg = `Error getting table schema: ${e instanceof Error ? e.message : String(e)}`
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
- \`list_files()\`: List lightweight metadata for frontend files and backend runnables. It does not include file or runnable content.
- \`get_frontend_file(path)\`: Get full content of a specific frontend file
- \`patch_file(path, old_string, new_string, replace_all?)\`: Make a quick exact text edit to an existing file. Use frontend paths like \`/index.tsx\` and inline backend paths like \`backend/listRecipes/main.ts\`.
- \`set_frontend_file(path, content)\`: Create or update a frontend file. Returns lint diagnostics.
- \`delete_frontend_file(path)\`: Delete a frontend file
- \`get_backend_runnable(key)\`: Get full configuration of a specific backend runnable
- \`set_backend_runnable(key, name, type, ...)\`: Create or update a backend runnable. Returns lint diagnostics.
- \`delete_backend_runnable(key)\`: Delete a backend runnable

### Linting
- \`lint()\`: Lint all files. Returns errors/warnings grouped by frontend/backend. Use this to check for issues after making changes.

## Quick Edits with patch_file

Use \`patch_file\` for small, localized edits when you can copy an exact snippet from the current file contents.

- \`old_string\` must be copied exactly from the current file contents
- Leave \`replace_all\` as false unless you intentionally want to update every exact match
- Never patch \`/wmill.d.ts\`; it is generated automatically from backend runnables
- Use \`set_frontend_file\` or \`set_backend_runnable\` instead for large rewrites, new files, or new runnables

### Discovery
- \`search_workspace(query, type)\`: Search workspace scripts and flows
- \`get_runnable_details(path, type)\`: Get details (summary, description, schema, content) of a specific script or flow
- \`search_hub_scripts(query)\`: Search hub scripts

### Data Tables
- \`list_datatables()\`: List configured datatables with schema and table names only. Does not include columns. Use this directly for table-list or available-tables summaries.
- \`get_datatable_table_schema(datatable_name, schema_name, table_name)\`: Get column definitions for one table. Do not call this for simple row counts or table-list summaries.
- \`exec_datatable_sql(datatable_name, sql, new_table?)\`: Execute SQL query on a datatable. Use for data exploration or modifications. When creating a new table, pass \`new_table: { schema, name }\` to register it in the app.

## Data Storage with Data Tables

**When the app needs to store or persist data, you MUST use datatables.** Datatables provide a managed PostgreSQL database that integrates seamlessly with Windmill apps, with near-zero setup and workspace-scoped access.

### Key Principles

1. **Always check existing tables first**: Use \`list_datatables()\` to see what tables are already available. If a suitable table exists, **always reuse it** rather than creating a new one. For dashboards that only show available tables or row counts, \`list_datatables()\` is enough. Only call \`get_datatable_table_schema()\` for tables whose column names/types you need.

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

**TypeScript (Bun) example**:
\`\`\`typescript
import * as wmill from 'windmill-client';

export async function main(user_id: string) {
  const sql = ${datatableCall};
  const user = await sql\`SELECT * FROM ${schemaPrefix}users WHERE id = \${user_id}\`.fetchOne();
  return user;
}
\`\`\`

**Python example**:
\`\`\`python
import wmill

def main(user_id: str):
    db = ${datatableCall}
    user = db.query('SELECT * FROM ${schemaPrefix}users WHERE id = $1', user_id).fetch_one()
    return user
\`\`\`

Use these examples for normal datatable access.

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

1. Use the smallest context needed. If the target file or runnable is clear, inspect only that item with \`get_frontend_file(path)\` or \`get_backend_runnable(key)\`.
2. Use \`list_files()\` only when the target is unclear or the request needs a broad app overview. \`list_files()\` returns metadata only, never file contents.
3. For small localized edits, prefer \`patch_file\`. Use \`set_frontend_file\` and \`set_backend_runnable\` for larger rewrites, new files, or new runnables. These return lint diagnostics.
4. Use \`lint()\` at the end to check for and fix any remaining errors

When creating a new app, use \`search_workspace\` or \`search_hub_scripts\` to find existing scripts/flows to reuse.
When the user mentions frontend files or backend runnables in context, only their identifiers are included. Use \`get_frontend_file\` or \`get_backend_runnable\` to inspect their contents before editing them or relying on implementation details.

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
		selectedContext && (selectedContext.inspectorElement || selectedContext.codeSelection)
	const hasAdditionalContext = additionalContext && additionalContext.length > 0

	if (hasSelectedContext || hasAdditionalContext) {
		content += `## SELECTED CONTEXT:\n`

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
					content += `\n- Frontend file: ${ctx.path}\n`
				} else if (ctx.type === 'app_backend_runnable') {
					content += `\n- Backend runnable: ${ctx.key}\n`
				} else if (ctx.type === 'app_datatable') {
					const datatableCtx = ctx as AppDatatableElement
					const tableRef = formatAppDatatableContextTitle(
						datatableCtx.datatableName,
						datatableCtx.schemaName,
						datatableCtx.tableName
					)
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
