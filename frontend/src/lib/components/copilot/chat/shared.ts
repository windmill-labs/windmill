import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/chat/completions.mjs'
import { get } from 'svelte/store'
import type { ContextElement } from './context'
import { workspaceStore } from '$lib/stores'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import { zodToJsonSchema } from 'zod-to-json-schema'
import { z } from 'zod'
import { ScriptService } from '$lib/gen'

type BaseDisplayMessage = {
	content: string
	contextElements?: ContextElement[]
	snapshot?: ExtendedOpenFlow
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
	isLoading?: boolean
	error?: string
	needsConfirmation?: boolean
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
		throw new Error(`Unknown tool call: ${functionName}`)
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
	toolCall: ChatCompletionMessageToolCall
	helpers: T
	toolCallbacks: ToolCallbacks
}): Promise<ChatCompletionMessageParam> {
	try {
		console.log('[processToolCall] Starting tool call:', toolCall.function.name, toolCall.id)
		const args = JSON.parse(toolCall.function.arguments || '{}')
		const tool = tools.find((t) => t.def.function.name === toolCall.function.name)
		
		console.log('[processToolCall] Found tool:', tool?.def.function.name, 'requiresConfirmation:', tool?.requiresConfirmation)
		console.log('[processToolCall] Has requestConfirmation callback:', !!toolCallbacks.requestConfirmation)
		
		// Check if tool requires confirmation
		const needsConfirmation = tool?.requiresConfirmation
		
		console.log('[processToolCall] Needs confirmation:', needsConfirmation)
		
		// Add the tool to the display with appropriate status
		toolCallbacks.setToolStatus(toolCall.id, 
			needsConfirmation ? 'Waiting for confirmation...' : `Preparing ${toolCall.function.name.replace('api_', '')}...`, 
			{
				parameters: args,
				isLoading: true,
				needsConfirmation: needsConfirmation,
			}
		)
		
		// If confirmation is needed and we have the callback, wait for it
		if (needsConfirmation && toolCallbacks.requestConfirmation) {
			console.log('[processToolCall] Requesting confirmation for:', toolCall.id)
			const confirmed = await toolCallbacks.requestConfirmation(toolCall.id)
			console.log('[processToolCall] Confirmation result:', confirmed)
			
			if (!confirmed) {
				toolCallbacks.setToolStatus(toolCall.id, 'Cancelled by user', {
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
			toolCallbacks.setToolStatus(toolCall.id, `Executing ${toolCall.function.name.replace('api_', '')}...`, {
				isLoading: true,
				needsConfirmation: false
			})
		}
		
		let result = ''
		try {
			// TODO: remove fake delay
			await new Promise((resolve) => setTimeout(resolve, 1000))
			result = await callTool({
				tools,
				functionName: toolCall.function.name,
				args,
				workspace: get(workspaceStore) ?? '',
				helpers,
				toolCallbacks,
				toolId: toolCall.id
			})
		} catch (err) {
			console.error(err)
			result =
				'Error while calling tool, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request'
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
			content:
				'Error while calling tool, MUST tell the user to check the browser console for more details, and then respond as much as possible to the original request'
		}
	}
}

export interface Tool<T> {
	def: ChatCompletionTool
	fn: (p: {
		args: any
		workspace: string
		helpers: T
		toolCallbacks: ToolCallbacks
		toolId: string
	}) => Promise<string>
	preAction?: (p: { toolCallbacks: ToolCallbacks; toolId: string }) => void
	requiresConfirmation?: boolean
}

export interface ToolCallbacks {
	setToolStatus: (id: string, content: string, metadata?: Partial<ToolDisplayMessage>) => void
	requestConfirmation?: (toolId: string) => Promise<boolean>
}

export function createToolDef(
	zodSchema: z.ZodSchema,
	name: string,
	description: string
): ChatCompletionTool {
	const schema = zodToJsonSchema(zodSchema, {
		name,
		target: 'openAi'
	})
	let parameters = schema.definitions![name] as FunctionParameters
	parameters = {
		...parameters,
		required: parameters.required ?? []
	}

	return {
		type: 'function',
		function: {
			strict: true,
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
		toolCallbacks.setToolStatus(
			toolId,
			'Searching for hub scripts related to "' + args.query + '"...'
		)
		const parsedArgs = searchHubScriptsSchema.parse(args)
		const scripts = await ScriptService.queryHubScripts({
			text: parsedArgs.query,
			kind: 'script'
		})
		toolCallbacks.setToolStatus(
			toolId,
			'Found ' + scripts.length + ' scripts in the hub related to "' + args.query + '"'
		)
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
