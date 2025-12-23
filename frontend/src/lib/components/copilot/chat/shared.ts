import type {
	ChatCompletionFunctionTool,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/chat/completions.mjs'

/**
 * Special module IDs used throughout the flow system
 */
export const SPECIAL_MODULE_IDS = {
	/** The flow input schema node */
	INPUT: 'Input',
	/** The preprocessor module that runs before the flow */
	PREPROCESSOR: 'preprocessor',
	/** The failure handler module */
	FAILURE: 'failure'
} as const
import { get } from 'svelte/store'
import type { CodePieceElement, ContextElement, FlowModuleCodePieceElement } from './context'
import { workspaceStore } from '$lib/stores'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import { z } from 'zod'
import { ScriptService, JobService, type CompletedJob, type FlowModule } from '$lib/gen'
import { scriptLangToEditorLang } from '$lib/scripts'
import { getCurrentModel } from '$lib/aiStore'
import { type editor as meditor } from 'monaco-editor'

// Prettify function for code arguments - extracts and formats code from JSON
function prettifyCodeArguments(content: string): string {
	let codeContent = content

	// If it's a JSON string, try to extract the code property
	if (typeof content === 'string' && content.trim().startsWith('{')) {
		try {
			const parsed = JSON.parse(content)
			if (parsed.code) {
				codeContent = parsed.code
			}
		} catch {
			// If JSON is incomplete during streaming, try to extract manually
			// Remove leading { "code": " or {"code":"
			codeContent = content.replace(/^\{\s*"code"\s*:\s*"/, '')
			// Remove trailing } if it exists
			codeContent = codeContent.replace(/"\s*}\s*$/, '')
		}
	}

	// Convert escaped newlines to actual newlines
	codeContent = codeContent.replace(/\\n/g, '\n')

	// Convert other common escape sequences
	codeContent = codeContent.replace(/\\t/g, '\t')
	codeContent = codeContent.replace(/\\"/g, '"')
	codeContent = codeContent.replace(/\\\\/g, '\\')

	return codeContent
}

// Prettify function for set_module_code - extracts code from moduleId/code JSON
function prettifySetModuleCode(content: string): string {
	let codeContent = content

	if (typeof content === 'string' && content.trim().startsWith('{')) {
		try {
			const parsed = JSON.parse(content)
			if (parsed.code) {
				codeContent = parsed.code
			}
		} catch {
			// If JSON is incomplete during streaming, try to extract code property manually
			const codeMatch = content.match(/"code"\s*:\s*"([\s\S]*?)(?:"\s*}?\s*$|$)/)
			if (codeMatch) {
				codeContent = codeMatch[1]
			}
		}
	}

	// Convert escape sequences
	codeContent = codeContent.replace(/\\n/g, '\n')
	codeContent = codeContent.replace(/\\t/g, '\t')
	codeContent = codeContent.replace(/\\"/g, '"')
	codeContent = codeContent.replace(/\\\\/g, '\\')

	return codeContent
}

// Prettify function for module value JSON - extracts the 'value' property and formats it
function prettifyModuleValue(content: string): string {
	try {
		const parsed = JSON.parse(content)
		// Extract just the 'value' property (the actual module definition)
		if (parsed.value) {
			return JSON.stringify(parsed.value, null, 2)
		}
		return JSON.stringify(parsed, null, 2)
	} catch {
		// If JSON is incomplete during streaming, try to extract the value property manually
		const valueMatch = content.match(/"value"\s*:\s*(\{[\s\S]*)$/)
		if (valueMatch) {
			let valueContent = valueMatch[1]
			// Try to parse and format the extracted value
			try {
				// Find the matching closing brace for the value object
				let braceCount = 0
				let endIndex = 0
				for (let i = 0; i < valueContent.length; i++) {
					if (valueContent[i] === '{') braceCount++
					else if (valueContent[i] === '}') braceCount--
					if (braceCount === 0) {
						endIndex = i + 1
						break
					}
				}
				if (endIndex > 0) {
					const valueJson = valueContent.substring(0, endIndex)
					const parsed = JSON.parse(valueJson)
					return JSON.stringify(parsed, null, 2)
				}
			} catch {
				// If parsing fails, just unescape and return the extracted value content
				valueContent = valueContent.replace(/\\n/g, '\n')
				valueContent = valueContent.replace(/\\t/g, '\t')
				valueContent = valueContent.replace(/\\"/g, '"')
				valueContent = valueContent.replace(/\\\\/g, '\\')
				return valueContent
			}
		}
		// Fallback: just unescape and return
		let result = content
		result = result.replace(/\\n/g, '\n')
		result = result.replace(/\\t/g, '\t')
		result = result.replace(/\\"/g, '"')
		result = result.replace(/\\\\/g, '\\')
		return result
	}
}

// Map of tool names to their prettify functions
export const TOOL_PRETTIFY_MAP: Record<string, (content: string) => string> = {
	edit_code: prettifyCodeArguments,
	set_module_code: prettifySetModuleCode,
	add_module: prettifyModuleValue,
	modify_module: prettifyModuleValue
}

export interface ContextStringResult {
	dbContext: string
	diffContext: string
	flowModuleContext: string
	hasDb: boolean
	hasDiff: boolean
	hasFlowModule: boolean
}

export const extractAllModules = (modules: FlowModule[]): FlowModule[] => {
	return modules.flatMap((m) => {
		if (m.value.type === 'forloopflow' || m.value.type === 'whileloopflow') {
			return [m, ...extractAllModules(m.value.modules)]
		}
		if (m.value.type === 'branchall') {
			return [m, ...extractAllModules(m.value.branches.flatMap((b) => b.modules))]
		}
		if (m.value.type === 'branchone') {
			return [
				m,
				...extractAllModules([...m.value.branches.flatMap((b) => b.modules), ...m.value.default])
			]
		}
		return [m]
	})
}

export const findModuleById = (modules: FlowModule[], moduleId: string): FlowModule | undefined => {
	for (const module of modules) {
		if (module.id === moduleId) {
			return module
		}
		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			const found = findModuleById(module.value.modules, moduleId)
			if (found) {
				return found
			}
		}
		if (module.value.type === 'branchall') {
			const allModules = module.value.branches.flatMap((b) => b.modules)
			const found = findModuleById(allModules, moduleId)
			if (found) {
				return found
			}
		}
		if (module.value.type === 'branchone') {
			const allModules = [
				...module.value.branches.flatMap((b) => b.modules),
				...module.value.default
			]
			const found = findModuleById(allModules, moduleId)
			if (found) {
				return found
			}
		}
	}
	return undefined
}

const applyCodePieceToCodeContext = (codePieces: CodePieceElement[], codeContext: string) => {
	let code = codeContext.split('\n')
	let shiftOffset = 0
	codePieces.sort((a, b) => a.startLine - b.startLine)
	for (const codePiece of codePieces) {
		code.splice(codePiece.endLine + shiftOffset, 0, '[#END]')
		code.splice(codePiece.startLine + shiftOffset - 1, 0, '[#START]')
		shiftOffset += 2
	}
	return code.join('\n')
}

export function applyCodePiecesToFlowModules(
	codePieces: FlowModuleCodePieceElement[],
	flowModules: FlowModule[]
): string {
	const moduleCodePieces = new Map<string, FlowModuleCodePieceElement[]>()
	for (const codePiece of codePieces) {
		const moduleId = codePiece.id
		if (!moduleCodePieces.has(moduleId)) {
			moduleCodePieces.set(moduleId, [])
		}
		moduleCodePieces.get(moduleId)!.push(codePiece)
	}

	// Clone modules to avoid mutation
	const modifiedModules = JSON.parse(JSON.stringify(flowModules))

	// Apply code pieces to each module
	for (const [moduleId, pieces] of moduleCodePieces) {
		const module = findModuleById(modifiedModules, moduleId)
		if (module && module.value.type === 'rawscript' && module.value.content) {
			module.value.content = applyCodePieceToCodeContext(
				pieces as unknown as CodePieceElement[],
				module.value.content
			)
		}
	}

	return JSON.stringify(modifiedModules, null, 2)
}

export function buildContextString(selectedContext: ContextElement[]): string {
	const dbTemplate = `- {title}: SCHEMA: \n{schema}\n`
	const codeTemplate = `
	- {title}:
	\`\`\`{language}
	{code}
	\`\`\`
	`

	let dbContext = 'DATABASES:\n'
	let diffContext = 'DIFF:\n'
	let flowModuleContext = 'FOCUSED FLOW MODULES IDS:\n'
	let codeContext = 'CODE:\n'
	let errorContext = `
	ERROR:
	{error}
	`
	let hasCode = false
	let hasDb = false
	let hasDiff = false
	let hasFlowModule = false
	let hasError = false

	let result = '\n\n'
	for (const context of selectedContext) {
		if (context.type === 'code') {
			hasCode = true
			codeContext += codeTemplate
				.replace('{title}', context.title)
				.replace('{language}', scriptLangToEditorLang(context.lang))
				.replace(
					'{code}',
					applyCodePieceToCodeContext(
						selectedContext.filter((c) => c.type === 'code_piece'),
						context.content
					)
				)
		} else if (context.type === 'error') {
			if (hasError) {
				throw new Error('Multiple error contexts provided')
			}
			hasError = true
			errorContext = errorContext.replace('{error}', context.content)
		} else if (context.type === 'db') {
			hasDb = true
			dbContext += dbTemplate
				.replace('{title}', context.title)
				.replace('{schema}', context.schema?.stringified ?? 'to fetch with get_db_schema')
			dbContext += '\n'
		} else if (context.type === 'diff') {
			hasDiff = true
			const diff = JSON.stringify(context.diff)
			diffContext += (diff.length > 3000 ? diff.slice(0, 3000) + '...' : diff) + '\n'
		} else if (context.type === 'flow_module') {
			hasFlowModule = true
			flowModuleContext += `${context.id}\n`
		}
	}

	if (hasCode) {
		result += '\n' + codeContext
	}
	if (hasError) {
		result += '\n' + errorContext
	}
	if (hasDb) {
		result += '\n' + dbContext
	}
	if (hasDiff) {
		result += '\n' + diffContext
	}
	if (hasFlowModule) {
		result += '\n' + flowModuleContext
	}

	return result
}

type BaseDisplayMessage = {
	content: string
	contextElements?: ContextElement[]
	snapshot?: { type: 'flow'; value: ExtendedOpenFlow } | { type: 'app'; value: number }
}

export type UserDisplayMessage = BaseDisplayMessage & {
	role: 'user'
	index: number // Used to match index with actual chat messages
	error?: boolean
}

export type ToolDisplayMessage = {
	role: 'tool'
	tool_call_id: string
	content: string
	parameters?: any
	result?: any
	logs?: string
	isLoading?: boolean
	error?: string
	needsConfirmation?: boolean
	showDetails?: boolean
	isStreamingArguments?: boolean
	toolName?: string
	showFade?: boolean
}

export type AssistantDisplayMessage = BaseDisplayMessage & {
	role: 'assistant'
}

export type DisplayMessage = UserDisplayMessage | ToolDisplayMessage | AssistantDisplayMessage

async function callTool<T>({
	tools,
	functionName,
	args,
	workspace,
	helpers,
	toolCallbacks,
	toolId
}: {
	tools: Tool<T>[]
	functionName: string
	args: any
	workspace: string
	helpers: T
	toolCallbacks: ToolCallbacks
	toolId: string
}): Promise<string> {
	const tool = tools.find((t) => t.def.function.name === functionName)
	if (!tool) {
		throw new Error(
			`Unknown tool call: ${functionName}. Probably not in the correct mode, use the change_mode tool to switch to the correct mode.`
		)
	}
	return tool.fn({ args, workspace, helpers, toolCallbacks, toolId })
}

export async function processToolCall<T>({
	tools,
	toolCall,
	helpers,
	toolCallbacks
}: {
	tools: Tool<T>[]
	toolCall: ChatCompletionMessageFunctionToolCall
	helpers: T
	toolCallbacks: ToolCallbacks
}): Promise<ChatCompletionMessageParam> {
	try {
		const args = JSON.parse(toolCall.function.arguments || '{}')
		const tool = tools.find((t) => t.def.function.name === toolCall.function.name)

		// Check if tool requires confirmation
		const needsConfirmation = tool?.requiresConfirmation

		toolCallbacks.setToolStatus(toolCall.id, {
			...(tool?.requiresConfirmation
				? { content: tool.confirmationMessage ?? 'Waiting for confirmation...' }
				: {}),
			parameters: args,
			isLoading: true,
			needsConfirmation: needsConfirmation,
			showDetails: tool?.showDetails
		})

		// If confirmation is needed and we have the callback, wait for it
		if (needsConfirmation && toolCallbacks.requestConfirmation) {
			const confirmed = await toolCallbacks.requestConfirmation(toolCall.id)

			if (!confirmed) {
				toolCallbacks.setToolStatus(toolCall.id, {
					content: 'Cancelled by user',
					isLoading: false,
					error: 'Tool execution was cancelled by user',
					needsConfirmation: false
				})
				return {
					role: 'tool' as const,
					tool_call_id: toolCall.id,
					content: 'Tool execution was cancelled by user'
				}
			}

			// Update status to executing after confirmation
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: true,
				needsConfirmation: false
			})
		}

		let result = ''
		try {
			result = await callTool({
				tools,
				functionName: toolCall.function.name,
				args,
				workspace: get(workspaceStore) ?? '',
				helpers,
				toolCallbacks,
				toolId: toolCall.id
			})
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false
			})
		} catch (err) {
			console.error(err)
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false,
				error: 'An error occurred while calling the tool'
			})
			const errorMessage =
				typeof err === 'object' && 'message' in err
					? err.message
					: typeof err === 'string'
						? err
						: 'An error occurred while calling the tool'
			result = `Error while calling tool: ${errorMessage}`
		}
		const toAdd = {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content: result
		}
		return toAdd
	} catch (err) {
		console.error(err)
		return {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content: 'Error while calling tool'
		}
	}
}

export interface Tool<T> {
	def: ChatCompletionFunctionTool
	fn: (p: {
		args: any
		workspace: string
		helpers: T
		toolCallbacks: ToolCallbacks
		toolId: string
	}) => Promise<string>
	preAction?: (p: { toolCallbacks: ToolCallbacks; toolId: string }) => void
	setSchema?: (helpers: any) => Promise<void>
	requiresConfirmation?: boolean
	confirmationMessage?: string
	showDetails?: boolean
	streamArguments?: boolean
	showFade?: boolean
}

export interface ToolCallbacks {
	setToolStatus: (id: string, metadata?: Partial<ToolDisplayMessage>) => void
	removeToolStatus: (id: string) => void
	requestConfirmation?: (toolId: string) => Promise<boolean>
}

export function createToolDef(
	zodSchema: z.ZodSchema,
	name: string,
	description: string,
	{ strict = true }: { strict?: boolean } = {} // we sometimes have to set strict to false for open ai models to avoid issues with complex properties
): ChatCompletionFunctionTool {
	// console.log('creating tool def for', name, zodSchema)
	let parameters = z.toJSONSchema(zodSchema)
	delete parameters.$schema
	if (!parameters.required) parameters.required = []

	return {
		type: 'function',
		function: {
			strict,
			name,
			description,
			parameters
		}
	}
}

const searchHubScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchHubScriptsToolDef = createToolDef(
	searchHubScriptsSchema,
	'search_hub_scripts',
	'Search for scripts in the hub'
)

export const createSearchHubScriptsTool = (withContent: boolean = false) => ({
	def: searchHubScriptsToolDef,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, {
			content: 'Searching for hub scripts related to "' + args.query + '"...'
		})
		const parsedArgs = searchHubScriptsSchema.parse(args)
		const scripts = await ScriptService.queryHubScripts({
			text: parsedArgs.query,
			kind: 'script'
		})
		toolCallbacks.setToolStatus(toolId, {
			content: 'Found ' + scripts.length + ' scripts in the hub related to "' + args.query + '"'
		})
		// if withContent, fetch scripts with their content, limit to 3 results
		const results = await Promise.all(
			scripts.slice(0, withContent ? 3 : undefined).map(async (s) => {
				let content = ''
				if (withContent) {
					content = await ScriptService.getHubScriptContentByPath({
						path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
					})
				}
				return {
					path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: s.summary,
					...(withContent ? { content } : {})
				}
			})
		)
		return JSON.stringify(results)
	}
})

/**
 * Recursively removes format: null or format: '' from a JSON schema object
 */
function removeNullFormats(schema: Record<string, any> | undefined): void {
	if (!schema || typeof schema !== 'object') {
		return
	}

	// Remove format if it's null or empty string
	if (schema.format === null || schema.format === '') {
		delete schema.format
	}

	// Recurse into properties
	if (schema.properties && typeof schema.properties === 'object') {
		for (const key of Object.keys(schema.properties)) {
			removeNullFormats(schema.properties[key])
		}
	}

	// Recurse into items (for arrays)
	if (schema.items) {
		removeNullFormats(schema.items)
	}

	// Recurse into additionalProperties if it's an object schema
	if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
		removeNullFormats(schema.additionalProperties)
	}

	// Recurse into allOf, anyOf, oneOf
	for (const key of ['allOf', 'anyOf', 'oneOf']) {
		if (Array.isArray(schema[key])) {
			for (const subSchema of schema[key]) {
				removeNullFormats(subSchema)
			}
		}
	}
}

export async function buildSchemaForTool(
	toolDef: ChatCompletionFunctionTool,
	schemaBuilder: () => Promise<FunctionParameters>
): Promise<boolean> {
	try {
		const schema = await schemaBuilder()

		// if schema properties contains values different from '^[a-zA-Z0-9_.-]{1,64}$'
		const invalidProperties = Object.keys(schema.properties ?? {}).filter(
			(key) => !/^[a-zA-Z0-9_.-]{1,64}$/.test(key)
		)
		if (invalidProperties.length > 0) {
			console.warn(`Invalid flow inputs schema: ${invalidProperties.join(', ')}`)
			throw new Error(`Invalid flow inputs schema: ${invalidProperties.join(', ')}`)
		}

		toolDef.function.parameters = { ...schema, additionalProperties: false }

		// recursively remove any format: null or format: '' (empty string) from schema
		removeNullFormats(toolDef.function.parameters)

		// OPEN AI models don't support strict mode well with schema with complex properties, so we disable it
		const model = getCurrentModel()
		if (model.provider === 'openai' || model.provider === 'azure_openai') {
			toolDef.function.strict = false
		}
		return true
	} catch (error) {
		console.error('Error building schema for tool', error)
		// fallback to schema with args as a JSON string
		toolDef.function.parameters = {
			type: 'object',
			properties: {
				args: { type: 'string', description: 'JSON string containing the arguments for the tool' }
			},
			additionalProperties: false,
			strict: false,
			required: ['args']
		}
		return false
	}
}

// Constants for result formatting
const MAX_RESULT_LENGTH = 12000
const MAX_LOG_LENGTH = 4000

export interface TestRunConfig {
	jobStarter: () => Promise<string>
	workspace: string
	toolCallbacks: ToolCallbacks
	toolId: string
	startMessage?: string
	contextName: 'script' | 'flow'
}

// Common job polling function
export async function pollJobCompletion(
	jobId: string,
	workspace: string,
	toolId: string,
	toolCallbacks: ToolCallbacks
): Promise<CompletedJob> {
	let attempts = 0
	const maxAttempts = 60
	let job: CompletedJob | null = null

	while (attempts < maxAttempts) {
		await new Promise((resolve) => setTimeout(resolve, 1000))
		attempts++

		try {
			const fetchedJob = await JobService.getJob({
				workspace: workspace,
				id: jobId,
				noLogs: false,
				noCode: true
			})

			if (fetchedJob.type === 'CompletedJob') {
				job = fetchedJob
				break
			}
		} catch (error) {
			if (attempts >= maxAttempts) {
				throw error
			}
		}
	}

	if (!job) {
		toolCallbacks.setToolStatus(toolId, {
			content: 'Test timed out',
			error: 'Execution timed out or failed to complete'
		})
		throw new Error('Test execution timed out after 60 seconds')
	}

	return job
}

// Helper function to extract code blocks from markdown text
export function extractCodeFromMarkdown(markdown: string): string[] {
	const codeBlocks: string[] = []

	// Matches: ```[language]\n[code]\n```
	const codeBlockRegex = /```(?:[a-z]+)?\n([\s\S]*?)```/g

	let match: RegExpExecArray | null = null
	while ((match = codeBlockRegex.exec(markdown)) !== null) {
		const code = match[1].trim()
		if (code) {
			codeBlocks.push(code)
		}
	}

	return codeBlocks
}

// Helper function to get the latest assistant message from display messages
export function getLatestAssistantMessage(displayMessages: DisplayMessage[]): string | undefined {
	// Iterate from the end to find the most recent assistant message
	for (let i = displayMessages.length - 1; i >= 0; i--) {
		const message = displayMessages[i]
		if (message.role === 'assistant' && message.content) {
			return message.content
		}
	}
	return undefined
}

// Helper function to extract error messages from job results
function getErrorMessage(result: unknown): string {
	if (typeof result === 'object' && result !== null && 'error' in result) {
		const error = (result as Record<string, unknown>).error
		if (typeof error === 'object' && error !== null && 'message' in error) {
			const message = (error as Record<string, unknown>).message as string
			if ('stack' in error) {
				return (message + '\n' + (error as Record<string, unknown>).stack) as string
			}
			return message
		}
		if (typeof error === 'string') {
			return error
		}
	}
	if (typeof result === 'string') {
		return result
	}
	return 'Unknown error'
}

// Build test run args based on the tool definition, if it contains a fallback schema
export async function buildTestRunArgs(
	args: any,
	toolDef: ChatCompletionFunctionTool
): Promise<any> {
	let parsedArgs = args
	// if the schema is the fallback schema, parse the args as a JSON string
	if (
		(toolDef.function.parameters as any).properties?.args?.description ===
		'JSON string containing the arguments for the tool'
	) {
		try {
			parsedArgs = JSON.parse(args.args)
		} catch (error) {
			console.error('Error parsing arguments for tool', error)
		}
	}
	return parsedArgs
}

// Main execution function for test runs
export async function executeTestRun(config: TestRunConfig): Promise<string> {
	try {
		config.toolCallbacks.setToolStatus(config.toolId, {
			content: config.startMessage || `Starting ${config.contextName} test...`
		})

		const jobId = await config.jobStarter()

		const contextName = config.contextName.charAt(0).toUpperCase() + config.contextName.slice(1)

		config.toolCallbacks.setToolStatus(config.toolId, {
			content: `${contextName} test started, waiting for completion...`
		})

		const job = await pollJobCompletion(
			jobId,
			config.workspace,
			config.toolId,
			config.toolCallbacks
		)

		config.toolCallbacks.setToolStatus(config.toolId, {
			content: `${contextName} test ${job.success ? 'completed successfully' : 'failed'}`,
			result: formatResult(job.result),
			logs: formatLogs(job.logs),
			...(job.success ? {} : { error: getErrorMessage(job.result) })
		})

		return formatResultSummary(job.result, job.logs, job.success)
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
		config.toolCallbacks.setToolStatus(config.toolId, {
			content: `Test execution failed`,
			error: errorMessage
		})
		throw new Error(`Failed to execute test run: ${errorMessage}`)
	}
}

function formatLogs(logs: string | undefined): undefined | string {
	if (logs && logs.trim()) {
		if (logs.length <= MAX_LOG_LENGTH) {
			return logs
		} else {
			return logs.slice(-MAX_LOG_LENGTH)
		}
	}
	return undefined
}

function formatResult(result: unknown): string {
	if (typeof result === 'string') {
		return result
	}
	return JSON.stringify(result, null, 2)
}

function formatResultSummary(result: unknown, logs: string | undefined, success: boolean): string {
	let resultSummary = ''
	resultSummary += `Result (${success ? 'SUCCESS' : 'FAILED'})\n\n`
	resultSummary += formatResult(result).slice(0, MAX_RESULT_LENGTH)
	resultSummary += '\n\nLogs:\n\n'
	resultSummary += formatLogs(logs) ?? 'No logs available'
	return resultSummary
}

// ============= Script/Flow Lint Types =============

/** Result of linting a script */
export interface ScriptLintResult {
	errorCount: number
	warningCount: number
	errors: meditor.IMarker[]
	warnings: meditor.IMarker[]
}

/** Format script lint result for display */
export function formatScriptLintResult(lintResult: ScriptLintResult): string {
	let response = ''
	const hasIssues = lintResult.errorCount > 0 || lintResult.warningCount > 0

	if (hasIssues) {
		if (lintResult.errorCount > 0) {
			response += `❌ **${lintResult.errorCount} error(s)** found that must be fixed:\n`
			for (const error of lintResult.errors) {
				response += `- Line ${error.startLineNumber}: ${error.message}\n`
			}
		}

		if (lintResult.warningCount > 0) {
			response += `\n⚠️ **${lintResult.warningCount} warning(s)** found:\n`
			for (const warning of lintResult.warnings) {
				response += `- Line ${warning.startLineNumber}: ${warning.message}\n`
			}
		}
	} else {
		response = '✅ No lint issues found.'
	}

	return response
}
