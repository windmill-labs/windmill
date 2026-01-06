import { OpenAI } from 'openai'
import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageFunctionToolCall
} from 'openai/resources/index.mjs'
import type {
	MessageParam,
	TextBlockParam,
	ToolUnion,
	ToolUseBlockParam,
	Tool as AnthropicTool,
	Message,
	RawMessageStreamEvent
} from '@anthropic-ai/sdk/resources'
import type { MessageStream } from '@anthropic-ai/sdk/lib/MessageStream'
import { getProviderAndCompletionConfig, workspaceAIClients } from '../lib'
import { processToolCall, type Tool, type ToolCallbacks } from './shared'

export async function getAnthropicCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionFunctionTool[]
): Promise<MessageStream> {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true })
	const { system, messages: anthropicMessages } = convertOpenAIToAnthropicMessages(messages)
	const anthropicTools = convertOpenAIToolsToAnthropic(tools)

	const anthropicClient = workspaceAIClients.getAnthropicClient()

	const anthropicParams = {
		model: config.model,
		max_tokens: config.max_tokens as number,
		messages: anthropicMessages,
		...(system && { system }),
		...(anthropicTools && { tools: anthropicTools }),
		...(typeof config.temperature === 'number' && { temperature: config.temperature })
	}

	const stream = anthropicClient.messages.stream(anthropicParams, {
		signal: abortController.signal,
		headers: {
			'X-Provider': provider,
			'anthropic-version': '2023-06-01',
			'X-Anthropic-SDK': 'true'
		}
	})

	return stream
}

export async function parseAnthropicCompletion(
	completion: MessageStream,
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	},
	messages: ChatCompletionMessageParam[],
	addedMessages: ChatCompletionMessageParam[],
	tools: Tool<any>[],
	helpers: any,
	abortController?: AbortController
): Promise<boolean> {
	let toolCallsToProcess: ChatCompletionMessageFunctionToolCall[] = []
	let error = null

	let currentStreamingTool:
		| { tempId: string; shouldStream: boolean; toolName: string }
		| undefined = undefined
	let accumulatedJson = ''

	completion.on('streamEvent', (event: RawMessageStreamEvent) => {
		if (event.type === 'content_block_start') {
			const block = event.content_block
			if (block.type === 'tool_use') {
				const toolName = block.name
				const toolId = block.id as string

				const tool = tools.find((t) => t.def.function.name === toolName)
				const shouldStream = tool?.streamArguments ?? false

				callbacks.onMessageEnd()

				// Reset accumulated JSON for new tool
				accumulatedJson = ''
				currentStreamingTool = { tempId: toolId, shouldStream, toolName }

				callbacks.setToolStatus(toolId, {
					isLoading: true,
					content: `Calling ${toolName}...`,
					toolName,
					isStreamingArguments: shouldStream,
					showFade: tool?.showFade,
					showDetails: tool?.showDetails
				})
			}
		}
	})

	completion.on('inputJson', (partialJson: string) => {
		if (currentStreamingTool?.shouldStream && currentStreamingTool.tempId) {
			// Accumulate the partial JSON
			accumulatedJson += partialJson

			// Try to parse and display
			try {
				const parsed = JSON.parse(accumulatedJson)
				callbacks.setToolStatus(currentStreamingTool.tempId, {
					parameters: parsed,
					isStreamingArguments: true,
					isLoading: true
				})
			} catch {
				// JSON incomplete, display as raw string
				callbacks.setToolStatus(currentStreamingTool.tempId, {
					parameters: accumulatedJson,
					isStreamingArguments: true,
					isLoading: true
				})
			}
		}
	})

	// Handle text streaming
	completion.on('text', (textDelta: string, _textSnapshot: string) => {
		callbacks.onNewToken(textDelta)
	})

	completion.on('message', (message: Message) => {
		for (const block of message.content) {
			if (block.type === 'text') {
				const text = block.text
				const assistantMessage = { role: 'assistant' as const, content: text }
				messages.push(assistantMessage)
				addedMessages.push(assistantMessage)
				callbacks.onMessageEnd()
			} else if (block.type === 'tool_use') {
				// Convert Anthropic tool calls to OpenAI format for compatibility
				toolCallsToProcess.push({
					id: block.id,
					type: 'function' as const,
					function: {
						name: block.name,
						arguments: JSON.stringify(block.input)
					}
				})
				// Preprocess tool if it has a preAction
				const tool = tools.find((t) => t.def.function.name === block.name)
				if (tool && tool.preAction) {
					tool.preAction({ toolCallbacks: callbacks, toolId: block.id })
				}
			}
		}

		// Clear temp tracking after processing
		currentStreamingTool = undefined
	})

	// Handle abort
	completion.on('abort', (e: any) => {
		// Check the AbortController's signal for the reason
		const abortReason = abortController?.signal.reason
		console.warn('Anthropic stream aborted:', {
			name: e?.name,
			message: e?.message,
			abortReason,
			wasAbortedByUser: abortReason === 'user_cancelled',
			signalAborted: abortController?.signal.aborted,
			cause: e?.cause,
			stack: e?.stack
		})
		error = e
	})

	// Handle errors
	completion.on('error', (e: any) => {
		console.error('Anthropic stream error:', {
			name: e?.name,
			message: e?.message,
			status: e?.status,
			headers: e?.headers,
			error: e?.error,
			cause: e?.cause,
			stack: e?.stack
		})
		error = e
	})

	// Wait for completion
	await completion.done()

	callbacks.onMessageEnd()

	if (error) {
		throw error
	}

	// Process tool calls if any
	if (toolCallsToProcess.length > 0) {
		const assistantWithTools = {
			role: 'assistant' as const,
			tool_calls: toolCallsToProcess
		}
		messages.push(assistantWithTools)
		addedMessages.push(assistantWithTools)

		// Process each tool call
		for (const toolCall of toolCallsToProcess) {
			const messageToAdd = await processToolCall({
				tools,
				toolCall,
				helpers,
				toolCallbacks: callbacks
			})
			messages.push(messageToAdd)
			addedMessages.push(messageToAdd)
		}
		return true // Continue the conversation loop
	}

	return false // End the conversation
}

export function convertOpenAIToAnthropicMessages(messages: ChatCompletionMessageParam[]): {
	system: TextBlockParam[] | undefined
	messages: MessageParam[]
} {
	let system: TextBlockParam[] | undefined
	const anthropicMessages: MessageParam[] = []

	for (const message of messages) {
		if (message.role === 'system') {
			const systemText =
				typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
			// Convert system to array format with cache_control for caching
			system = [
				{
					type: 'text',
					text: systemText,
					cache_control: { type: 'ephemeral' }
				}
			]
			continue
		}

		if (message.role === 'user') {
			anthropicMessages.push({
				role: 'user',
				content:
					typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
			})
		} else if (message.role === 'assistant') {
			const content: (TextBlockParam | ToolUseBlockParam)[] = []

			if (message.content) {
				content.push({
					type: 'text',
					text:
						typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
				})
			}

			if (message.tool_calls) {
				for (const toolCall of message.tool_calls) {
					if (toolCall.type !== 'function') continue
					let input = {}
					try {
						input = JSON.parse(toolCall.function.arguments || '{}')
					} catch (e) {
						console.error('Failed to parse tool call arguments', e)
					}
					content.push({
						type: 'tool_use',
						id: toolCall.id,
						name: toolCall.function.name,
						input
					})
				}
			}

			if (content.length > 0) {
				anthropicMessages.push({
					role: 'assistant',
					content: content.length === 1 && content[0].type === 'text' ? content[0].text : content
				})
			}
		} else if (message.role === 'tool') {
			// Tool results must be in user messages in Anthropic format
			anthropicMessages.push({
				role: 'user',
				content: [
					{
						type: 'tool_result',
						tool_use_id: message.tool_call_id,
						content:
							typeof message.content === 'string'
								? message.content
								: JSON.stringify(message.content)
					}
				]
			})
		}
	}

	// Add cache_control to the last message content blocks
	if (anthropicMessages.length > 0) {
		const lastMessage = anthropicMessages[anthropicMessages.length - 1]
		if (Array.isArray(lastMessage.content)) {
			// Add cache_control to the last content block
			if (lastMessage.content.length > 0) {
				const lastBlock = lastMessage.content[lastMessage.content.length - 1]
				if (lastBlock.type === 'text') {
					lastBlock.cache_control = { type: 'ephemeral' }
				}
			}
		} else if (typeof lastMessage.content === 'string') {
			// Convert string content to array format with cache_control
			lastMessage.content = [
				{
					type: 'text',
					text: lastMessage.content,
					cache_control: { type: 'ephemeral' }
				}
			]
		}
	}

	return { system, messages: anthropicMessages }
}

export function convertOpenAIToolsToAnthropic(
	tools?: OpenAI.Chat.Completions.ChatCompletionFunctionTool[]
): ToolUnion[] | undefined {
	if (!tools || tools.length === 0) return undefined

	const anthropicTools: ToolUnion[] = tools.map((tool) => ({
		name: tool.function.name,
		description: tool.function.description,
		input_schema: (tool.function.parameters || {
			type: 'object',
			properties: {}
		}) as AnthropicTool.InputSchema
	}))

	// Add cache_control to the last tool to cache all tool definitions
	if (anthropicTools.length > 0) {
		anthropicTools[anthropicTools.length - 1].cache_control = { type: 'ephemeral' }
	}

	return anthropicTools
}
