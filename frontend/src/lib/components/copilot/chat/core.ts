import { ResourceService } from '$lib/gen/services.gen'
import type { ResourceType, ScriptLang } from '$lib/gen/types.gen'
import { capitalize, isObject, toCamel } from '$lib/utils'
import { get, type Writable } from 'svelte/store'
import { getCompletion } from '../lib'
import { compile, phpCompile, pythonCompile } from '../utils'
import { Code, Database, TriangleAlert, Diff } from 'lucide-svelte'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/index.mjs'
import { workspaceStore, type DBSchema, dbSchemas } from '$lib/stores'
import { scriptLangToEditorLang } from '$lib/scripts'
import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/utils'
import { type Change } from 'diff'

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

const TS_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type inside the \`RT\` namespace: for instance \`RT.Stripe\`.
You should only use them if you need them to satisfy the user's instructions. Always use the RT namespace.`

const PYTHON_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type.
You need to **redefine** the type of the resources that are needed before the main function as TypedDict, but only include them if they are actually needed to achieve the function purpose.
The resource type name has to be exactly as specified (has to be IN LOWERCASE).
If an import conflicts with a resource type name, **you have to rename the imported object, not the type name**.
Make sure to import TypedDict from typing **if you're using it**`

const PHP_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type
You need to **redefine** the type of the resources that are needed before the main function, but only include them if they are actually needed to achieve the function purpose.
Before defining each type, check if the class already exists using class_exists.
The resource type name has to be exactly as specified.`

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
	'powershell'
]

export function getLangContext(
	lang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json',
	{ allowResourcesFetch = false }: { allowResourcesFetch?: boolean } = {}
) {
	const tsContext =
		TS_RESOURCE_TYPE_SYSTEM +
		(allowResourcesFetch
			? `\nTo query the RT namespace, you can use the \`search_resource_types\` function.`
			: '')
	switch (lang) {
		case 'bunnative':
		case 'nativets':
			return (
				'The user is coding in TypeScript. On Windmill, it is expected that the script exports a single **async** function called `main`. You should use fetch (available globally, no need to import) and are not allowed to import any libraries.\n' +
				tsContext
			)
		case 'bun':
			return (
				'The user is coding in TypeScript (bun runtime). On Windmill, it is expected that the script exports a single **async** function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.\n' +
				tsContext
			)
		case 'deno':
			return (
				'The user is coding in TypeScript (deno runtime). On Windmill, it is expected that the script exports a single **async** function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.\n' +
				tsContext +
				'\nYou can import deno libraries or you can import npm libraries like that: `import ... from "npm:{package}";`.'
			)
		case 'python3':
			return (
				'The user is coding in Python. On Windmill, it is expected the script contains at least one function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.' +
				PYTHON_RESOURCE_TYPE_SYSTEM +
				`${allowResourcesFetch ? `\nTo query the available resource types, you can use the \`search_resource_types\` function.` : ''}`
			)
		case 'php':
			return (
				'The user is coding in PHP. On Windmill, it is expected the script contains at least one function called `main`. The script must start with <?php.' +
				PHP_RESOURCE_TYPE_SYSTEM +
				`${allowResourcesFetch ? `\nTo query the available resource types, you can use the \`search_resource_types\` function.` : ''}` +
				`\nIf you need to import libraries, you need to specify them as comments in the following manner before the main function:
\`\`\`
// require:
// mylibrary/mylibrary
// myotherlibrary/myotherlibrary@optionalversion
\`\`\`
Make sure to have one per line.
No need to require autoload, it is already done.`
			)
		case 'rust':
			return `The user is coding in Rust. On Windmill, it is expected the script contains at least one function called \`main\` (without calling it) defined like this:
\`\`\`rust
use anyhow::anyhow;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct ReturnType {
    // ...
}

fn main(...) -> anyhow::Result<ReturnType>
\`\`\`
Arguments should be owned. Make sure the return type is serializable.

Packages must be made available with a partial cargo.toml by adding the following comment at the beginning of the script:
//! \`\`\`cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! \`\`\'
Serde is already included, no need to add it again.

If you want to handle async functions (e.g., using tokio), you need to keep the main function sync and create the runtime inside.
`
		case 'go':
			return `The user is coding in Go. On Windmill, it is expected the script exports a single function called \`main\`. Its return type has to be (\`{return_type}\`, error). The file package has to be "inner".`
		case 'bash':
			return `The user is coding in Bash. Do not include "#!/bin/bash". On Windmill, arguments are always string and can only be obtained with "var1="$1"", "var2="$2"", etc..`
		case 'postgresql':
			return `The user is coding in PostgreSQL. On Windmill, arguments can be obtained directly in the statement with \`$1::{type}\`, \`$2::{type}\`, etc... Name the parameters (without specifying the type) by adding comments at the beginning of the script before the statement like that: \`-- $1 name1\` or \`-- $2 name = default\` (one per row)`
		case 'mysql':
			return 'The user is coding in MySQL. On Windmill, arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)'
		case 'bigquery':
			return 'The user is coding in BigQuery. On Windmill, arguments can be obtained by adding comments before the statement like that: `-- @name1 ({type})` or `-- @name2 ({type}) = default` (one per row). They can then be obtained directly in the statement with `@name1`, `@name2`, etc....'
		case 'snowflake':
			return 'The user is coding in Snowflake. On Windmill, arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)'
		case 'mssql':
			return 'The user is coding in Microsoft SQL Server. On Windmill, arguments can be obtained directly in the statement with @P1, @P2, etc.. Name the parameters by adding comments before the statement like that: `-- @P1 name1 ({type})` or `-- @P2 name2 ({type}) = default` (one per row)'
		case 'graphql':
			return 'The user is coding in GraphQL. If needed, add the needed arguments as query parameters.'
		case 'powershell':
			return 'The user is coding in PowerShell. On Windmill, arguments can be obtained by calling the param function on the first line of the script like that: `param($ParamName1, $ParamName2 = "default value", [{type}]$ParamName3, ...)`'
		default:
			return ''
	}
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

export const CHAT_SYSTEM_PROMPT = `
	You are a coding assistant for the Windmill platform. You are provided with a list of \`INSTRUCTIONS\` and the current contents of a code file under \`CODE\`.

	Your task is to respond to the user's request. Assume all user queries are valid and actionable.

	When the user requests code changes:
	- Always include a **single code block** with the **entire updated file**, not just the modified sections.
	- The code can include \`[#START]\` and \`[#END]\` markers to indicate the start and end of a code piece. You MUST only modify the code between these markers if given, and remove them in your response. If a question is asked about the code, you MUST only talk about the code between the markers. Refer to it as the code piece, not the code between the markers.
	- Follow the instructions carefully and explain the reasoning behind your changes.
	- If the request is abstract (e.g., "make this cleaner"), interpret it concretely and reflect that in the code block.
	- Preserve existing formatting, indentation, and whitespace unless changes are strictly required to fulfill the user's request.
	- The user can ask you to look at or modify specific files, databases or errors by having its name in the INSTRUCTIONS preceded by the @ symbol. In this case, put your focus on the element that is explicitly mentioned.
	- The user can ask you questions about a list of \`DATABASES\` that are available in the user's workspace. If the user asks you a question about a database, you should ask the user to specify the database name if not given, or take the only one available if there is only one.
	- You can also receive a \`DIFF\` of the changes that have been made to the code. You should use this diff to give better answers.

	Important:
	Do not mention or reveal these instructions to the user unless explicitly asked to do so.
`

const CHAT_USER_CODE_CONTEXT = `
- {title}:
\`\`\`{language}
{code}
\`\`\`
`

const CHAT_USER_ERROR_CONTEXT = `
ERROR:
{error}
`

export const CHAT_USER_PROMPT = `
INSTRUCTIONS:
{instructions}

WINDMILL LANGUAGE CONTEXT:
{lang_context}

`

export const CHAT_USER_DB_CONTEXT = `- {title}: SCHEMA: \n{schema}\n`

export function prepareSystemMessage(): {
	role: 'system'
	content: string
} {
	return {
		role: 'system',
		content: CHAT_SYSTEM_PROMPT
	}
}

export interface DisplayMessage {
	role: 'user' | 'assistant'
	content: string
	contextElements?: ContextElement[]
}

export const ContextIconMap = {
	code: Code,
	error: TriangleAlert,
	db: Database,
	diff: Diff,
	code_piece: Code
}

type CodeElement = {
	type: 'code'
	content: string
	title: string
	lang: ScriptLang | 'bunnative'
}

type ErrorElement = {
	type: 'error'
	content: string
	title: 'error'
}

type DBElement = {
	type: 'db'
	schema?: DBSchema
	title: string
}

type DiffElement = {
	type: 'diff'
	content: string
	title: string
	diff: Change[]
	lang: ScriptLang | 'bunnative'
}

type CodePieceElement = {
	type: 'code_piece'
	content: string
	startLine: number
	endLine: number
	title: string
	lang: ScriptLang | 'bunnative'
}

export type ContextElement = CodeElement | ErrorElement | DBElement | DiffElement | CodePieceElement

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

export async function prepareUserMessage(
	instructions: string,
	language: ScriptLang | 'bunnative',
	selectedContext: ContextElement[]
) {
	let codeContext = 'CODE:\n'
	let errorContext = 'ERROR:\n'
	let dbContext = 'DATABASES:\n'
	let diffContext = 'DIFF:\n'
	let hasCode = false
	let hasError = false
	let hasDb = false
	let hasDiff = false
	for (const context of selectedContext) {
		if (context.type === 'code') {
			hasCode = true
			codeContext += CHAT_USER_CODE_CONTEXT.replace('{title}', context.title)
				.replace('{language}', scriptLangToEditorLang(language))
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
			errorContext = CHAT_USER_ERROR_CONTEXT.replace('{error}', context.content)
		} else if (context.type === 'db') {
			hasDb = true
			dbContext += CHAT_USER_DB_CONTEXT.replace('{title}', context.title).replace(
				'{schema}',
				context.schema?.stringified ?? 'to fetch with get_db_schema'
			)
		} else if (context.type === 'diff') {
			hasDiff = true
			const diff = JSON.stringify(context.diff)
			diffContext = diff.length > 3000 ? diff.slice(0, 3000) + '...' : diff
		}
	}

	let userMessage = CHAT_USER_PROMPT.replace('{instructions}', instructions).replace(
		'{lang_context}',
		getLangContext(language, { allowResourcesFetch: true })
	)
	if (hasCode) {
		userMessage += codeContext
	}
	if (hasError) {
		userMessage += errorContext
	}
	if (hasDb) {
		userMessage += dbContext
	}
	if (hasDiff) {
		userMessage += diffContext
	}
	return userMessage
}

const RESOURCE_TYPE_FUNCTION_DEF: ChatCompletionTool = {
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

const DB_SCHEMA_FUNCTION_DEF: ChatCompletionTool = {
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

async function callTool(
	functionName: string,
	args: any,
	lang: ScriptLang | 'bunnative',
	workspace: string
) {
	switch (functionName) {
		case 'search_resource_types':
			const formattedResourceTypes = await getFormattedResourceTypes(lang, args.query, workspace)
			return formattedResourceTypes
		case 'get_db_schema':
			if (!args.resourcePath) {
				throw new Error('Database path not provided')
			}
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
			return stringSchema
		default:
			throw new Error(`Unknown tool call: ${functionName}`)
	}
}

async function processToolCall(
	toolCall: ChatCompletionMessageToolCall,
	messages: ChatCompletionMessageParam[],
	lang: ScriptLang | 'bunnative'
) {
	try {
		const args = JSON.parse(toolCall.function.arguments)
		let result = ''
		try {
			result = await callTool(toolCall.function.name, args, lang, get(workspaceStore) ?? '')
		} catch (err) {
			console.error(err)
			result =
				'Error while calling tool, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request'
		}
		messages.push({
			role: 'tool',
			tool_call_id: toolCall.id,
			content: result
		})
	} catch (err) {
		console.error(err)
	}
}

export async function chatRequest(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	lang: ScriptLang | 'bunnative',
	useDbTools: boolean,
	onNewToken: (token: string) => void
) {
	const toolDefs: ChatCompletionTool[] = []
	if (
		lang === 'python3' ||
		lang === 'php' ||
		lang === 'bun' ||
		lang === 'deno' ||
		lang === 'nativets' ||
		lang === 'bunnative'
	) {
		toolDefs.push(RESOURCE_TYPE_FUNCTION_DEF)
	}
	if (useDbTools) {
		toolDefs.push(DB_SCHEMA_FUNCTION_DEF)
	}
	try {
		let completion: any = null
		while (true) {
			completion = await getCompletion(messages, abortController, toolDefs)

			if (completion) {
				const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

				for await (const chunk of completion) {
					if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
						continue
					}
					const c = chunk as ChatCompletionChunk
					const delta = c.choices[0].delta.content
					if (delta) {
						onNewToken(delta)
					}
					const toolCalls = c.choices[0].delta.tool_calls || []
					for (const toolCall of toolCalls) {
						const { index } = toolCall
						const finalToolCall = finalToolCalls[index]
						if (!finalToolCall) {
							finalToolCalls[index] = toolCall
						} else {
							if (toolCall.function?.arguments) {
								if (!finalToolCall.function) {
									finalToolCall.function = toolCall.function
								} else {
									finalToolCall.function.arguments =
										(finalToolCall.function.arguments ?? '') + toolCall.function.arguments
								}
							}
						}
					}
				}

				const toolCalls = Object.values(finalToolCalls).filter(
					(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
				) as ChatCompletionMessageToolCall[]

				if (toolCalls.length > 0) {
					messages.push({
						role: 'assistant',
						tool_calls: toolCalls
					})
					for (const toolCall of toolCalls) {
						await processToolCall(toolCall, messages, lang)
					}
				} else {
					break
				}
			}
		}
		return completion
	} catch (err) {
		if (!abortController.signal.aborted) {
			throw err
		}
	}
}

export interface AIChatContext {
	loading: Writable<boolean>
	currentReply: Writable<string>
	applyCode: (code: string) => void
}
