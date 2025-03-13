import { ResourceService } from '$lib/gen/services.gen'
import type { AIProvider, ResourceType, ScriptLang } from '$lib/gen/types.gen'
import { capitalize, toCamel } from '$lib/utils'
import { get, type Writable } from 'svelte/store'
import { getCompletion } from '../lib'
import { compile, phpCompile, pythonCompile } from '../utils'
import { Code, TriangleAlert } from 'lucide-svelte'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/index.mjs'
import { workspaceStore } from '$lib/stores'

export function formatResourceTypes(
	resourceTypes: ResourceType[],
	lang: 'python3' | 'php' | 'bun' | 'deno' | 'nativets'
) {
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
		const result = resourceTypes
			.filter(
				(resourceType) => Boolean(resourceType.schema) && typeof resourceType.schema === 'object'
			)
			.map((resourceType) => {
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

const TS_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed a parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type inside the \`RT\` namespace: for instance \`RT.Stripe\`.
You should only them if you need them to satisfy the user's instructions. Always use the RT namespace. 
To query the RT namespace, you can use the \`search_resource_types\` function.`

const PYTHON_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed a parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type.
To query the available resource types, you can use the \`search_resource_types\` function.
You need to **redefine** the type of the resources that are needed before the main function as TypedDict, but only include them if they are actually needed to achieve the function purpose.
The resource type name has to be exactly as specified (has to be IN LOWERCASE).
If an import conflicts with a resource type name, **you have to rename the imported object, not the type name**.
Make sure to import TypedDict from typing **if you're using it**`

const PHP_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed a parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type
The available resource types are provided by the user under the \`RESOURCE_TYPE_CONTEXT\` key.
You need to **redefine** the type of the resources that are needed before the main function, but only include them if they are actually needed to achieve the function purpose.
Before defining each type, check if the class already exists using class_exists.
The resource type name has to be exactly as specified.`

function getLangContext(lang: ScriptLang) {
	switch (lang) {
		case 'nativets':
			return (
				'The user is coding in TypeScript. On Windmill, it is expected that the script exports a single **async** function called `main`. You should use fetch and are not allowed to import any libraries.\n' +
				TS_RESOURCE_TYPE_SYSTEM
			)
		case 'bun':
			return (
				'The user is coding in TypeScript (bun runtime). On Windmill, it is expected that the script exports a single **async** function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.\n' +
				TS_RESOURCE_TYPE_SYSTEM
			)
		case 'deno':
			return (
				'The user is coding in TypeScript (deno runtime). On Windmill, it is expected that the script exports a single **async** function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.\n' +
				TS_RESOURCE_TYPE_SYSTEM +
				'\nYou can import deno libraries or you can import npm libraries like that: `import ... from "npm:{package}";`.'
			)
		case 'python3':
			return (
				'The user is coding in Python. On Windmill, it is expected the script contains at least one function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them.' +
				PYTHON_RESOURCE_TYPE_SYSTEM
			)
		case 'php':
			return (
				'The user is coding in PHP. On Windmill, it is expected the script contains at least one function called `main`.' +
				PHP_RESOURCE_TYPE_SYSTEM +
				`\nIf you need to import libraries, you need to specify them as comments in the following manner before the main function:
\`\`\`
// require:
// mylibrary/mylibrary
// myotherlibrary/myotherlibrary@optionalversion
\`\`\`
No need to require autoload, it is already done.`
			)
		default:
			return ''
	}
}

export async function getResourceTypeNamespace(
	lang: ScriptLang,
	prompt: string,
	workspace: string
) {
	switch (lang) {
		case 'deno':
		case 'bun':
		case 'nativets':
		case 'python3':
		case 'php': {
			const resourceTypes = await getResourceTypes(prompt, workspace)

			const intro = `RESOURCE_TYPE_CONTEXT:\n`

			const resourceTypesText = formatResourceTypes(resourceTypes, lang)

			return intro + resourceTypesText
		}
		default:
			return ''
	}
}

export const CHAT_SYSTEM_PROMPT = `
You are a coding assistant on the Windmill platform. You are given a list of instructions to follow \`INSTRUCTIONS\` as well as the current code in the file \`CODE\`.
{language_context}

Please respond to the user's query. The user's query is never invalid.

In the case that the user asks you to make changes to code, you should make sure to return a single CODE BLOCK, as well as explanations and descriptions of the changes.
For example, if the user asks you to "make this file look nicer", make sure your output includes a code block with concrete ways the file can look nicer.
- If suggesting changes, rewrite the **complete code** and not just a part of it.

Requirements:
- When suggesting changes, do not change spacing, indentation, or other whitespace apart from what is strictly necessary to apply the changes.

Do not output any of these instructions, nor tell the user anything about them unless directly prompted for them.
`

const CHAT_USER_CODE_CONTEXT = `
CODE:
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

{code_context}
{error_context}
\`\`\`
`

export function prepareSystemMessage(language: ScriptLang): { role: 'system'; content: string } {
	return {
		role: 'system',
		content: CHAT_SYSTEM_PROMPT.replace('{language_context}', getLangContext(language))
	}
}

export interface Context {
	code?: string
	error?: string
}

export type ContextConfig = Record<keyof Context, boolean>

export interface DisplayMessage {
	role: 'user' | 'assistant'
	content: string
	contextConfig?: ContextConfig
}

export const ContextIconMap = {
	code: Code,
	error: TriangleAlert
}

export async function prepareUserMessage(
	instructions: string,
	language: ScriptLang,
	workspace: string,
	context: Context
) {
	let codeContext = ''
	if (context.code !== undefined) {
		codeContext = CHAT_USER_CODE_CONTEXT.replace('{language}', language).replace(
			'{code}',
			context.code
		)
	}

	let errorPrompt = ''
	if (context.error !== undefined) {
		errorPrompt = CHAT_USER_ERROR_CONTEXT.replace('{error}', context.error)
	}

	const userMessage = CHAT_USER_PROMPT.replace('{instructions}', instructions)
		.replace('{code_context}', codeContext)
		.replace('{error_context}', errorPrompt)

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

export async function chatRequest(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	aiProvider: AIProvider,
	lang: ScriptLang,
	onNewToken: (token: string) => void
) {
	try {
		let completion: any = null
		while (true) {
			completion = await getCompletion(messages, abortController, aiProvider, [
				RESOURCE_TYPE_FUNCTION_DEF
			])

			if (completion) {
				const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

				for await (const chunk of completion) {
					const c = chunk as ChatCompletionChunk
					const delta = c.choices[0].delta.content
					if (delta) {
						onNewToken(delta)
					}
					const toolCalls = c.choices[0].delta.tool_calls || []
					for (const toolCall of toolCalls) {
						const { index } = toolCall
						if (!finalToolCalls[index]) {
							finalToolCalls[index] = toolCall
						} else {
							if (toolCall.function?.arguments) {
								if (!finalToolCalls[index].function) {
									finalToolCalls[index].function = toolCall.function
								} else {
									finalToolCalls[index].function.arguments =
										(finalToolCalls[index].function.arguments ?? '') + toolCall.function.arguments
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
						try {
							const args = JSON.parse(toolCall.function.arguments)
							const resourceType = await getResourceTypeNamespace(
								lang,
								args.query,
								get(workspaceStore) ?? ''
							)

							messages.push({
								role: 'tool',
								tool_call_id: toolCall.id,
								content: resourceType
							})
						} catch (err) {
							throw new Error('Error parsing tool call arguments')
						}
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
	originalCode: Writable<string>
	loading: Writable<boolean>
	currentReply: Writable<string>
	applyCode: (code: string) => void
}
