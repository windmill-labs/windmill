import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionSystemMessageParam,
	ChatCompletionTool
} from 'openai/resources/chat/completions.mjs'
import { get, type Writable } from 'svelte/store'
import type { ContextElement } from './context'
import { getCompletion } from '../lib'
import { workspaceStore } from '$lib/stores'

export interface AIChatContext {
	loading: Writable<boolean>
	currentReply: Writable<string>
	canApplyCode: () => boolean
	applyCode: (code: string) => void
}

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

async function processToolCall<T>({
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
}

export interface ToolCallbacks {
	onToolCall: (id: string, content: string) => void
	onFinishToolCall: (id: string, content: string) => void
}

export async function chatRequest<T>({
	systemMessage,
	messages,
	abortController,
	tools,
	helpers,
	callbacks
}: {
	systemMessage: ChatCompletionSystemMessageParam
	messages: ChatCompletionMessageParam[]
	abortController: AbortController
	tools: Tool<T>[]
	helpers: T
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	}
}) {
	try {
		let completion: any = null
		while (true) {
			completion = await getCompletion(
				[systemMessage, ...messages],
				abortController,
				tools.map((t) => t.def)
			)

			if (completion) {
				const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

				let answer = ''
				for await (const chunk of completion) {
					if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
						continue
					}
					const c = chunk as ChatCompletionChunk
					const delta = c.choices[0].delta.content
					if (delta) {
						answer += delta
						callbacks.onNewToken(delta)
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

				if (answer) {
					messages.push({ role: 'assistant', content: answer })
				}

				callbacks.onMessageEnd()

				const toolCalls = Object.values(finalToolCalls).filter(
					(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
				) as ChatCompletionMessageToolCall[]

				if (toolCalls.length > 0) {
					messages.push({
						role: 'assistant',
						tool_calls: toolCalls.map((t) => ({
							...t,
							function: {
								...t.function,
								arguments: t.function.arguments || '{}'
							}
						}))
					})
					for (const toolCall of toolCalls) {
						await processToolCall({ tools, toolCall, messages, helpers, toolCallbacks: callbacks })
					}
				} else {
					break
				}
			}
		}
		return messages
	} catch (err) {
		if (!abortController.signal.aborted) {
			throw err
		} else {
			return messages
		}
	}
}
