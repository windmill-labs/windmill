import { OpenAI } from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type {
	MessageParam,
	TextBlockParam,
	ToolUnion,
	ToolUseBlockParam,
	Tool
} from '@anthropic-ai/sdk/resources'

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
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
): ToolUnion[] | undefined {
	if (!tools || tools.length === 0) return undefined

	const anthropicTools: ToolUnion[] = tools.map((tool) => ({
		name: tool.function.name,
		description: tool.function.description,
		input_schema: (tool.function.parameters || {
			type: 'object',
			properties: {}
		}) as Tool.InputSchema
	}))

	// Add cache_control to the last tool to cache all tool definitions
	if (anthropicTools.length > 0) {
		anthropicTools[anthropicTools.length - 1].cache_control = { type: 'ephemeral' }
	}

	return anthropicTools
}

export async function* convertAnthropicStreamToOpenAI(
	model: string,
	stream: Stream<MessageStreamEvent>
): AsyncIterable<OpenAI.Chat.Completions.ChatCompletionChunk> {
	let currentToolCall: { id: string; name: string; args: string } | null = null
	const messageId = `chatcmpl-${Date.now()}`

	try {
		for await (const event of stream) {
			switch (event.type) {
				case 'message_start':
					yield {
						id: messageId,
						object: 'chat.completion.chunk',
						created: Math.floor(Date.now() / 1000),
						model,
						choices: [{ index: 0, delta: { role: 'assistant' }, finish_reason: null }]
					}
					break

				case 'content_block_start':
					if (event.content_block?.type === 'tool_use') {
						currentToolCall = {
							id: event.content_block.id,
							name: event.content_block.name,
							args: ''
						}
					}
					// Text and thinking blocks start here but don't need special handling
					break

				case 'content_block_delta':
					switch (event.delta?.type) {
						case 'text_delta':
							yield {
								id: messageId,
								object: 'chat.completion.chunk',
								created: Math.floor(Date.now() / 1000),
								model,
								choices: [{ index: 0, delta: { content: event.delta.text }, finish_reason: null }]
							}
							break

						case 'input_json_delta':
							if (currentToolCall) {
								currentToolCall.args += event.delta.partial_json
							}
							break

						case 'thinking_delta':
							// Skip thinking deltas - they're internal reasoning
							break

						default:
							// Handle unknown delta types gracefully
							break
					}
					break

				case 'content_block_stop':
					if (currentToolCall) {
						// Emit completed tool call
						yield {
							id: messageId,
							object: 'chat.completion.chunk',
							created: Math.floor(Date.now() / 1000),
							model,
							choices: [
								{
									index: 0,
									delta: {
										tool_calls: [
											{
												index: 0,
												id: currentToolCall.id,
												type: 'function' as const,
												function: { name: currentToolCall.name, arguments: currentToolCall.args }
											}
										]
									},
									finish_reason: null
								}
							]
						}
						currentToolCall = null
					}
					break

				case 'message_delta':
					const finishReason =
						event.delta?.stop_reason === 'end_turn'
							? 'stop'
							: event.delta?.stop_reason === 'tool_use'
								? 'tool_calls'
								: event.delta?.stop_reason === 'max_tokens'
									? 'length'
									: null

					yield {
						id: messageId,
						object: 'chat.completion.chunk',
						created: Math.floor(Date.now() / 1000),
						model,
						choices: [{ index: 0, delta: {}, finish_reason: finishReason }]
					}
					break

				case 'message_stop':
					// Stream completed successfully
					break

				case 'ping':
					// Ignore ping events
					break

				case 'error':
					// Handle error events
					throw new Error(`Anthropic stream error: ${JSON.stringify(event.error)}`)

				default:
					// Handle unknown event types gracefully as per Anthropic docs
					console.debug('Unknown Anthropic stream event type:', event.type)
					break
			}
		}
	} catch (error) {
		console.error('Error processing Anthropic stream:', error)
		throw error
	}
}
