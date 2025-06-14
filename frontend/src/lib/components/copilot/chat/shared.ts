import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionTool
} from 'openai/resources/chat/completions.mjs'
import { get } from 'svelte/store'
import type { ContextElement } from './context'
import { workspaceStore } from '$lib/stores'

export type DisplayMessage =
	| {
			role: 'user' | 'assistant'
			content: string
			contextElements?: ContextElement[]
	  }
	| {
			role: 'tool'
			tool_call_id: string
			content: string
	  }

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
	messages,
	helpers,
	toolCallbacks
}: {
	tools: Tool<T>[]
	toolCall: ChatCompletionMessageToolCall
	messages: ChatCompletionMessageParam[]
	helpers: T
	toolCallbacks: ToolCallbacks
}) {
	try {
		const args = JSON.parse(toolCall.function.arguments || '{}')
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
}

export interface ToolCallbacks {
	setToolStatus: (id: string, content: string) => void
}
