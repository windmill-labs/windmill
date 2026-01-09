import { ResourceService, JobService } from '$lib/gen/services.gen'
import type { AIProvider, AIProviderModel, ResourceType, ScriptLang } from '$lib/gen/types.gen'
import { capitalize, isObject, toCamel } from '$lib/utils'
import { get } from 'svelte/store'
import { compile, phpCompile, pythonCompile } from '../../utils'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionFunctionTool,
	ChatCompletionUserMessageParam
} from 'openai/resources/index.mjs'
import { type DBSchema, dbSchemas } from '$lib/stores'
import type { ContextElement } from '../context'
import {
	createSearchHubScriptsTool,
	type Tool,
	executeTestRun,
	buildTestRunArgs,
	buildContextString,
	type ScriptLintResult,
	formatScriptLintResult
} from '../shared'
import { setupTypeAcquisition, type DepsToGet } from '$lib/ata'
import { getModelContextWindow } from '../../lib'
import type { ReviewChangesOpts } from '../monaco-adapter'
import { getCurrentModel } from '$lib/aiStore'
import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
import { getScriptPrompt } from '$system_prompts'

// Score threshold for npm packages search filtering
const SCORE_THRESHOLD = 1000
// percentage of the context window for documentation of npm packages
const DOCS_CONTEXT_PERCENTAGE = 1
// percentage of the context window for types of npm packages
const TYPES_CONTEXT_PERCENTAGE = 1
// TODO: Explore this again when we have better diff-based edit providers
export const DIFF_BASED_EDIT_PROVIDERS: AIProvider[] = []

export function formatResourceTypes(
	allResourceTypes: ResourceType[],
	lang: 'python3' | 'php' | 'bun' | 'deno' | 'nativets' | 'bunnative'
) {
	const resourceTypes = allResourceTypes.filter(
		(rt) => isObject(rt.schema) && 'properties' in rt.schema && isObject(rt.schema.properties)
	)
	if (lang === 'python3') {
		const result = resourceTypes.map((resourceType) => {
			return `class ${resourceType.name}(TypedDict):\n${pythonCompile(resourceType.schema as any)}`
		})
		return '\n**Make sure to rename conflicting imported modules**\n' + result.join('\n\n')
	} else if (lang === 'php') {
		const result = resourceTypes.map((resourceType) => {
			return `class ${toCamel(capitalize(resourceType.name))} {\n${phpCompile(
				resourceType.schema as any
			)}\n}`
		})
		return '\n' + result.join('\n\n')
	} else {
		let resultStr = 'namespace RT {\n'
		const result = resourceTypes.map((resourceType) => {
			return `  type ${toCamel(capitalize(resourceType.name))} = ${compile(
				resourceType.schema as any
			).replaceAll('\n', '\n  ')}`
		})
		return resultStr + result.join('\n\n') + '\n}'
	}
}

async function getResourceTypes(prompt: string, workspace: string) {
	const resourceTypes = await ResourceService.queryResourceTypes({
		workspace: workspace,
		text: prompt,
		limit: 5
	})

	return resourceTypes
}

export const SUPPORTED_CHAT_SCRIPT_LANGUAGES = [
	'bunnative',
	'nativets',
	'bun',
	'deno',
	'python3',
	'php',
	'rust',
	'go',
	'bash',
	'postgresql',
	'mysql',
	'bigquery',
	'snowflake',
	'mssql',
	'graphql',
	'powershell',
	'csharp',
	'java',
	'duckdb'
]

export function getLangContext(
	lang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json',
	{
		allowResourcesFetch = false,
		isPreprocessor = false
	}: { allowResourcesFetch?: boolean; isPreprocessor?: boolean; isFailure?: boolean } = {}
): string {
	// Get base language context from centralized prompts
	let context = getScriptPrompt(lang)

	// Add tool usage instructions for applicable languages
	if (['python3', 'php', 'bun', 'deno', 'nativets', 'bunnative'].includes(lang)) {
		if (allowResourcesFetch) {
			context += '\n\nTo query available resource types, use the `search_resource_types` tool.'
		}
	}

	// Note preprocessor function naming if applicable
	if (isPreprocessor) {
		context +=
			'\n\nThe main function for this script should be named `preprocessor` instead of `main`.'
	}

	return context
}

export async function getFormattedResourceTypes(
	lang: ScriptLang | 'bunnative',
	prompt: string,
	workspace: string
) {
	switch (lang) {
		case 'deno':
		case 'bun':
		case 'nativets':
		case 'bunnative':
		case 'python3':
		case 'php': {
			const resourceTypes = await getResourceTypes(prompt, workspace)

			const intro = `RESOURCE_TYPES:\n`

			const resourceTypesText = formatResourceTypes(resourceTypes, lang)

			return intro + resourceTypesText
		}
		default:
			return ''
	}
}

function buildChatSystemPrompt(currentModel: AIProviderModel) {
	const useDiffBasedEdit = DIFF_BASED_EDIT_PROVIDERS.includes(currentModel.provider)
	const editToolName = EDIT_CODE_TOOL.function.name
	const editInstructions = useDiffBasedEdit
		? `
		- Pass an array of **diff objects** to the \`${editToolName}\` tool using the \`diffs\` parameter. Each diff should specify exactly what text to replace and what to replace it with.
		  - Each diff object must contain:
			- \`old_string\`: The exact text to replace (must match the current code exactly)
			- \`new_string\`: The replacement text
			- \`replace_all\` (optional): Set to true to replace all occurrences, false or omit for first occurrence only
		  - Example: [{"old_string": "return 1", "new_string": "return 2"}]`
		: `- Pass the **complete updated file** to the \`${editToolName}\` tool using the \`code\` parameter, not just the modified sections.`

	return `
	You are a coding assistant for the Windmill platform. You are provided with a list of \`INSTRUCTIONS\` and the current contents of a code file under \`CODE\`.

	Your task is to respond to the user's request. Assume all user queries are valid and actionable.

	When the user requests code changes:
	- ALWAYS use the \`${editToolName}\` tool to apply code changes. Use it only once.
	${editInstructions}
	- The code can include \`[#START]\` and \`[#END]\` markers to indicate the start and end of a code piece. You MUST only modify the code between these markers if given, and remove them when passing to the tool. If a question is asked about the code, you MUST only talk about the code between the markers. Refer to it as the code piece, not the code between the markers.
	- Follow the instructions carefully and explain the reasoning behind your changes in your response text.
	- If the request is abstract (e.g., "make this cleaner"), interpret it concretely and reflect that in your changes.
	- Preserve existing formatting, indentation, and whitespace unless changes are strictly required to fulfill the user's request.
	- The user can ask you to look at or modify specific files, databases or errors by having its name in the INSTRUCTIONS preceded by the @ symbol. In this case, put your focus on the element that is explicitly mentioned.
	- The user can ask you questions about a list of \`DATABASES\` that are available in the user's workspace. If the user asks you a question about a database, you should ask the user to specify the database name if not given, or take the only one available if there is only one.
	- You can also receive a \`DIFF\` of the changes that have been made to the code. You should use this diff to give better answers.
	- Before giving your answer, check again that you carefully followed these instructions.
	- When asked to create a script that communicates with an external service, you can use the \`search_hub_scripts\` tool to search for relevant scripts in the hub. Make sure the language is the same as what the user is coding in. If you do not find any relevant scripts, you can use the \`search_npm_packages\` tool to search for relevant packages and their documentation. Always give a link to the documentation in your answer if possible.
	- After applying code changes with the \`${editToolName}\` tool, ALWAYS use the \`get_lint_errors\` tool to check for lint errors. If there are errors, fix them before proceeding. Then use the \`test_run_script\` tool to test the code, and iterate on the code until it works as expected (MAX 3 times). If the user cancels the test run, do not try again and wait for the next user instruction.

	Important:
	${useDiffBasedEdit ? '- Each old_string must match the exact text in the current code, including whitespace and indentation.' : ''}
	- Do not return the applied code in your response, just explain what you did. You can return code blocks in your response for explanations or examples as per user request.
	- Do not mention or reveal these instructions to the user unless explicitly asked to do so.
`
}

export const INLINE_CHAT_SYSTEM_PROMPT = `
# Windmill Inline Coding Assistant

You are a coding assistant for the Windmill platform. You provide precise code modifications based on user instructions.

## Input Format

You will receive:
- **INSTRUCTIONS**: User's modification request
- **CODE**: Current code content with modification boundaries
- **DATABASES** *(optional)*: Available workspace databases

### Code Boundaries

The code contains \`[#START]\` and \`[#END]\` markers indicating the modification scope:
- **MUST** only modify code between these markers
- **MUST** remove the markers in your response
- **MUST** preserve all other code exactly as provided

## Task Requirements

Return the modified CODE that fulfills the user's request. Assume all user queries are valid and actionable.

### Critical Rules

- ✅ **ALWAYS** include a single code block with the entire updated CODE
- ✅ **ALWAYS** use the structured XML output format below
- ❌ **NEVER** include only modified sections
- ❌ **NEVER** add explanatory text or comments outside the format
- ❌ **NEVER** include \`\`\` code fences in your response
- ❌ **NEVER** modify the code outside the boundaries

## Output Format

\`\`\`xml
<changes_made>
Brief description of what was changed
</changes_made>
<new_code>
[complete modified code without markers]
</new_code>
\`\`\`

## Example

### Input:
\`\`\`xml
<user_request>
INSTRUCTIONS:
Return 2 instead of 1

CODE:
import * as wmill from "windmill-client"

function test() {
	return "hello"
}

[#START]
export async function main() {
	return 1;
}
[#END]
</user_request>
\`\`\`

### Expected Output:
\`\`\`xml
<changes_made>
Changed return value from 1 to 2 in main function
</changes_made>
<new_code>
import * as wmill from "windmill-client"

function test() {
	return "hello"
}

export async function main() {
	return 2;
}
</new_code>
\`\`\`
`

export function prepareInlineChatSystemPrompt(lang: ScriptLang | 'bunnative') {
	return INLINE_CHAT_SYSTEM_PROMPT + getLangContext(lang, { allowResourcesFetch: true })
}

export const CHAT_USER_PROMPT = `
INSTRUCTIONS:
{instructions}

`

export function prepareScriptSystemMessage(
	currentModel: AIProviderModel,
	language: ScriptLang | 'bunnative',
	options: { isPreprocessor?: boolean; allowResourcesFetch?: boolean } = {},
	customPrompt?: string
): ChatCompletionSystemMessageParam {
	let content = buildChatSystemPrompt(currentModel)

	// Add language context to the system prompt
	const langContext = getLangContext(language, { allowResourcesFetch: true, ...options })
	content += `\n\nWINDMILL LANGUAGE CONTEXT:\n${langContext}`

	// If there's a custom prompt, append it to the system prompt
	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareScriptTools(
	currentModel: AIProviderModel,
	language: ScriptLang | 'bunnative',
	context: ContextElement[]
): Tool<ScriptChatHelpers>[] {
	const tools: Tool<ScriptChatHelpers>[] = []
	if (['python3', 'php', 'bun', 'deno', 'nativets', 'bunnative'].includes(language)) {
		tools.push(resourceTypeTool)
	}
	if (context.some((c) => c.type === 'db')) {
		tools.push(dbSchemaTool)
	}
	if (['bun', 'deno'].includes(language)) {
		tools.push(createSearchHubScriptsTool(true))
		tools.push(searchNpmPackagesTool)
	}
	const useDiffBasedEdit = DIFF_BASED_EDIT_PROVIDERS.includes(currentModel.provider)
	if (useDiffBasedEdit) {
		tools.push(editCodeToolWithDiff)
	} else {
		tools.push(editCodeTool)
	}
	tools.push(testRunScriptTool)
	tools.push(getLintErrorsTool)
	return tools
}

export function prepareScriptUserMessage(
	instructions: string,
	selectedContext: ContextElement[]
): ChatCompletionUserMessageParam {
	let userMessage = CHAT_USER_PROMPT.replace('{instructions}', instructions)
	const contextInstructions = buildContextString(selectedContext)
	userMessage += contextInstructions

	return {
		role: 'user',
		content: userMessage
	}
}

const RESOURCE_TYPE_FUNCTION_DEF: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'search_resource_types',
		description: 'Finds and returns resource types that are relevant to the specified query',
		parameters: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description:
						'The query to search for, e.g. specific integration (e.g. "stripe") or a specific feature (e.g. "send emails")'
				}
			},
			required: ['query'],
			additionalProperties: false
		},
		strict: true
	}
}

const DB_SCHEMA_FUNCTION_DEF: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'get_db_schema',
		description: 'Gets the schema of the database',
		parameters: {
			type: 'object',
			properties: {
				resourcePath: { type: 'string', description: 'The path of the database resource' }
			},
			required: ['resourcePath']
		}
	}
}

export const MAX_SCHEMA_LENGTH = 100000 * 3.5

async function formatDBSchema(dbSchema: DBSchema) {
	let { stringified } = dbSchema
	if (dbSchema.lang === 'graphql') {
		if (stringified.length > MAX_SCHEMA_LENGTH) {
			stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
		}
		return 'GRAPHQL SCHEMA:\n' + stringified
	} else {
		if (stringified.length > MAX_SCHEMA_LENGTH) {
			stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
		}
		return (
			'DATABASE SCHEMA (each column is in the format [name, type, required, default?]):\n' +
			stringified
		)
	}
}

export interface ScriptChatHelpers {
	getScriptOptions: () => {
		code: string
		lang: ScriptLang | 'bunnative'
		path: string
		args: Record<string, any>
	}
	applyCode: (code: string, opts?: ReviewChangesOpts) => Promise<void>
	/** Get lint errors from the Monaco editor */
	getLintErrors?: () => ScriptLintResult
}

export const resourceTypeTool: Tool<ScriptChatHelpers> = {
	def: RESOURCE_TYPE_FUNCTION_DEF,
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		toolCallbacks.setToolStatus(toolId, {
			content: 'Searching resource types for "' + args.query + '"...'
		})
		const lang = helpers.getScriptOptions().lang
		const formattedResourceTypes = await getFormattedResourceTypes(lang, args.query, workspace)
		toolCallbacks.setToolStatus(toolId, {
			content: 'Retrieved resource types for "' + args.query + '"'
		})
		return formattedResourceTypes
	}
}

// Generic DB schema tool factory that can be used by both script and flow modes
export function createDbSchemaTool<T>(): Tool<T> {
	return {
		def: DB_SCHEMA_FUNCTION_DEF,
		fn: async ({ args, workspace, toolCallbacks, toolId }) => {
			if (!args.resourcePath) {
				throw new Error('Database path not provided')
			}
			toolCallbacks.setToolStatus(toolId, {
				content: 'Getting database schema for ' + args.resourcePath + '...'
			})
			const resource = await ResourceService.getResource({
				workspace: workspace,
				path: args.resourcePath
			})
			const newDbSchemas = {}
			await getDbSchemas(
				resource.resource_type,
				args.resourcePath,
				workspace,
				newDbSchemas,
				(error) => {
					console.error(error)
				}
			)
			dbSchemas.update((schemas) => ({ ...schemas, ...newDbSchemas }))
			const dbs = get(dbSchemas)
			const db = dbs[args.resourcePath]
			if (!db) {
				throw new Error('Database not found')
			}
			const stringSchema = await formatDBSchema(db)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved database schema for ' + args.resourcePath
			})
			return stringSchema
		}
	}
}

export const dbSchemaTool: Tool<ScriptChatHelpers> = createDbSchemaTool<ScriptChatHelpers>()

type PackageSearchQuery = {
	package: {
		name: string
		version: string
		links: {
			npm: string
			homepage: string
			repository: string
			bugs: string
		}
	}
	searchScore: number
}

type PackageSearchResult = {
	package: string
	documentation: string
	types: string
}

const packagesSearchCache = new Map<string, PackageSearchResult[]>()
export async function searchExternalIntegrationResources(args: { query: string }): Promise<string> {
	try {
		if (packagesSearchCache.has(args.query)) {
			return JSON.stringify(packagesSearchCache.get(args.query))
		}

		const result = await fetch(`https://registry.npmjs.org/-/v1/search?text=${args.query}&size=2`)
		const data = await result.json()
		const filtered = data.objects.filter(
			(r: PackageSearchQuery) => r.searchScore >= SCORE_THRESHOLD
		)

		const model = getCurrentModel()
		const modelContextWindow = getModelContextWindow(model.model)
		const results: PackageSearchResult[] = await Promise.all(
			filtered.map(async (r: PackageSearchQuery) => {
				let documentation = ''
				let types = ''
				try {
					const docResponse = await fetch(`https://unpkg.com/${r.package.name}/readme.md`)
					const docLimit = Math.floor((modelContextWindow * DOCS_CONTEXT_PERCENTAGE) / 100)
					documentation = await docResponse.text()
					documentation = documentation.slice(0, docLimit)
				} catch (error) {
					console.error('Error getting documentation for package:', error)
					documentation = ''
				}
				try {
					const typesResponse = await fetchNpmPackageTypes(r.package.name, r.package.version)
					const typesLimit = Math.floor((modelContextWindow * TYPES_CONTEXT_PERCENTAGE) / 100)
					types = typesResponse.types.slice(0, typesLimit)
				} catch (error) {
					console.error('Error getting types for package:', error)
					types = ''
				}
				return {
					package: r.package.name,
					documentation: documentation,
					types: types
				}
			})
		)
		packagesSearchCache.set(args.query, results)
		return JSON.stringify(results)
	} catch (error) {
		console.error('Error searching external integration resources:', error)
		return 'Error searching external integration resources'
	}
}

const SEARCH_NPM_PACKAGES_TOOL: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'search_npm_packages',
		description: 'Search for npm packages and their documentation',
		parameters: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description: 'The query to search for'
				}
			},
			required: ['query']
		}
	}
}

export const searchNpmPackagesTool: Tool<ScriptChatHelpers> = {
	def: SEARCH_NPM_PACKAGES_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, { content: 'Searching for relevant packages...' })
		const result = await searchExternalIntegrationResources(args)
		toolCallbacks.setToolStatus(toolId, { content: 'Retrieved relevant packages' })
		return result
	}
}

export async function fetchNpmPackageTypes(
	packageName: string,
	version: string = 'latest'
): Promise<{ success: boolean; types: string; error?: string }> {
	try {
		const typeDefinitions = new Map<string, string>()

		const ata = setupTypeAcquisition({
			projectName: 'NPM-Package-Types',
			depsParser: () => [],
			root: '',
			delegate: {
				receivedFile: (code: string, path: string) => {
					if (path.endsWith('.d.ts')) {
						typeDefinitions.set(path, code)
					}
				},
				localFile: () => {}
			}
		})

		const depsToGet: DepsToGet = [
			{
				raw: packageName,
				module: packageName,
				version: version
			}
		]

		await ata(depsToGet)

		if (typeDefinitions.size === 0) {
			return {
				success: false,
				types: '',
				error: `No type definitions found for ${packageName}`
			}
		}

		const formattedTypes = Array.from(typeDefinitions.entries())
			.map(([path, content]) => `// ${path}\n${content}`)
			.join('\n\n')

		return {
			success: true,
			types: formattedTypes
		}
	} catch (error) {
		console.error('Error fetching NPM package types:', error)
		return {
			success: false,
			types: '',
			error: `Error fetching package types: ${error instanceof Error ? error.message : 'Unknown error'}`
		}
	}
}

const EDIT_CODE_TOOL: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'edit_code',
		description: 'Apply code changes to the current script in the editor',
		parameters: {
			type: 'object',
			properties: {
				code: {
					type: 'string',
					description: 'The complete updated code for the entire script file.'
				}
			},
			additionalProperties: false,
			strict: true,
			required: ['code']
		}
	}
}

const EDIT_CODE_TOOL_WITH_DIFF: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'edit_code',
		description: 'Apply code changes to the current script in the editor',
		parameters: {
			type: 'object',
			properties: {
				diffs: {
					type: 'array',
					description: 'Array of diff objects to apply to the code',
					items: {
						type: 'object',
						properties: {
							old_string: {
								type: 'string',
								description: 'The exact text to replace (must match the current code exactly)'
							},
							new_string: {
								type: 'string',
								description: 'The new text to replace the old_string with'
							},
							replace_all: {
								type: 'boolean',
								description:
									'If true, replace all occurrences of old_string. If false or omitted, only replace the first occurrence.'
							}
						},
						required: ['old_string', 'new_string'],
						additionalProperties: false
					}
				}
			},
			additionalProperties: false,
			strict: true,
			required: ['diffs']
		}
	}
}

const TEST_RUN_SCRIPT_TOOL: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'test_run_script',
		description: 'Execute a test run of the current script in the editor',
		parameters: {
			type: 'object',
			properties: {
				args: { type: 'string', description: 'JSON string containing the arguments for the tool' }
			},
			additionalProperties: false,
			strict: false,
			required: ['args']
		}
	}
}

const GET_LINT_ERRORS_TOOL: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		name: 'get_lint_errors',
		description:
			'Get lint errors and warnings from the current script in the editor. Use this after making code changes to check for issues.',
		parameters: {
			type: 'object',
			properties: {},
			additionalProperties: false,
			strict: true,
			required: []
		}
	}
}

export const editCodeToolWithDiff: Tool<ScriptChatHelpers> = {
	def: EDIT_CODE_TOOL_WITH_DIFF,
	streamArguments: true,
	showDetails: true,
	showFade: true,
	fn: async function ({ args, helpers, toolCallbacks, toolId }) {
		const scriptOptions = helpers.getScriptOptions()

		if (!scriptOptions) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'No script available to edit',
				error: 'No script found in current context'
			})
			throw new Error(
				'No script code available to edit. Please ensure you have a script open in the editor.'
			)
		}

		if (!args.diffs || !Array.isArray(args.diffs)) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Invalid diffs provided',
				error: 'Diffs parameter is required and must be an array'
			})
			throw new Error('Diffs parameter is required and must be an array')
		}

		toolCallbacks.setToolStatus(toolId, { content: 'Applying code changes...' })

		try {
			// Save old code
			const oldCode = scriptOptions.code

			// Apply diffs sequentially
			let updatedCode = oldCode
			for (const [index, diff] of args.diffs.entries()) {
				const { old_string, new_string, replace_all = false } = diff

				if (!updatedCode.includes(old_string)) {
					throw new Error(`Diff at index ${index}: old_string "${old_string}" not found in code`)
				}

				if (replace_all) {
					updatedCode = updatedCode.replaceAll(old_string, new_string)
				} else {
					updatedCode = updatedCode.replace(old_string, new_string)
				}
			}

			// Apply the code changes directly
			await helpers.applyCode(updatedCode, { applyAll: true, mode: 'apply' })

			// Show revert mode
			await helpers.applyCode(oldCode, { mode: 'revert' })

			toolCallbacks.setToolStatus(toolId, {
				content: `Code changes applied`,
				result: 'Success'
			})
			return `Applied changes to the script editor.`
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
			toolCallbacks.setToolStatus(toolId, {
				content: 'Failed to apply code changes',
				error: errorMessage
			})
			throw new Error(`Failed to apply code changes: ${errorMessage}`)
		}
	}
}

export const editCodeTool: Tool<ScriptChatHelpers> = {
	def: EDIT_CODE_TOOL,
	streamArguments: true,
	showDetails: true,
	showFade: true,
	fn: async function ({ args, helpers, toolCallbacks, toolId }) {
		const scriptOptions = helpers.getScriptOptions()

		if (!scriptOptions) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'No script available to edit',
				error: 'No script found in current context'
			})
			throw new Error(
				'No script code available to edit. Please ensure you have a script open in the editor.'
			)
		}

		if (!args.code || typeof args.code !== 'string') {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Invalid code provided',
				error: 'Code parameter is required and must be a string'
			})
			throw new Error('Code parameter is required and must be a string')
		}

		try {
			// Save old code
			const oldCode = scriptOptions.code

			// Apply the code changes directly
			await helpers.applyCode(args.code, { applyAll: true, mode: 'apply' })

			// Show revert mode
			await helpers.applyCode(oldCode, { mode: 'revert' })

			toolCallbacks.setToolStatus(toolId, {
				content: 'Code changes applied',
				result: 'Success'
			})
			return 'Code has been applied to the script editor.'
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
			toolCallbacks.setToolStatus(toolId, {
				content: 'Failed to apply code changes',
				error: errorMessage
			})
			throw new Error(`Failed to apply code changes: ${errorMessage}`)
		}
	}
}

export const testRunScriptTool: Tool<ScriptChatHelpers> = {
	def: TEST_RUN_SCRIPT_TOOL,
	fn: async function ({ args, workspace, helpers, toolCallbacks, toolId }) {
		const scriptOptions = helpers.getScriptOptions()

		if (!scriptOptions) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'No script available to test',
				error: 'No script found in current context'
			})
			throw new Error(
				'No script code available to test. Please ensure you have a script open in the editor.'
			)
		}

		const parsedArgs = await buildTestRunArgs(args, this.def)

		return executeTestRun({
			jobStarter: () =>
				JobService.runScriptPreview({
					workspace: workspace,
					requestBody: {
						path: scriptOptions.path,
						content: scriptOptions.code,
						args: parsedArgs,
						language: scriptOptions.lang as ScriptLang
					}
				}),
			workspace,
			toolCallbacks,
			toolId,
			startMessage: 'Running test...',
			contextName: 'script'
		})
	},
	requiresConfirmation: true,
	confirmationMessage: 'Run script test',
	showDetails: true
}

export const getLintErrorsTool: Tool<ScriptChatHelpers> = {
	def: GET_LINT_ERRORS_TOOL,
	fn: async function ({ helpers, toolCallbacks, toolId }) {
		toolCallbacks.setToolStatus(toolId, { content: 'Getting lint errors...' })

		if (!helpers.getLintErrors) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Lint errors not available',
				error: 'getLintErrors helper is not available in this context'
			})
			return 'Lint errors are not available in this context. The editor may not support lint error reporting.'
		}

		const lintResult = helpers.getLintErrors()

		const status =
			lintResult.errorCount > 0
				? `Found ${lintResult.errorCount} error(s)`
				: lintResult.warningCount > 0
					? `Found ${lintResult.warningCount} warning(s)`
					: 'No issues found'

		toolCallbacks.setToolStatus(toolId, { content: status })

		return formatScriptLintResult(lintResult)
	}
}
