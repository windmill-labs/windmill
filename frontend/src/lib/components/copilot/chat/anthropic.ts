import { OpenAI } from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

export interface AnthropicMessage {
	role: 'user' | 'assistant'
	content:
		| string
		| Array<
				| {
						type: 'text'
						text: string
						cache_control?: { type: 'ephemeral' }
				  }
				| {
						type: 'tool_use'
						id: string
						name: string
						input: any
				  }
				| {
						type: 'tool_result'
						tool_use_id: string
						content: string
				  }
		  >
}

export interface AnthropicRequest {
	model: string
	max_tokens: number
	messages: AnthropicMessage[]
	system?: string | Array<{ type: 'text'; text: string; cache_control?: { type: 'ephemeral' } }>
	tools?: Array<{
		name: string
		description?: string
		input_schema: any
		cache_control?: { type: 'ephemeral' }
	}>
	stream?: boolean
	temperature?: number
}

export interface AnthropicStreamEvent {
	type: string
	[key: string]: any
}

export function convertOpenAIToAnthropicMessages(messages: ChatCompletionMessageParam[]): {
	system?: string | Array<{ type: 'text'; text: string; cache_control?: { type: 'ephemeral' } }>
	messages: AnthropicMessage[]
} {
	let system:
		| string
		| Array<{ type: 'text'; text: string; cache_control?: { type: 'ephemeral' } }>
		| undefined
	const anthropicMessages: AnthropicMessage[] = []

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
			const content: any[] = []

			if (message.content) {
				content.push({
					type: 'text',
					text:
						typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
				})
			}

			if (message.tool_calls) {
				for (const toolCall of message.tool_calls) {
					content.push({
						type: 'tool_use',
						id: toolCall.id,
						name: toolCall.function.name,
						input: JSON.parse(toolCall.function.arguments || '{}')
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
						tool_use_id: message.tool_call_id || '',
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
):
	| Array<{
			name: string
			description?: string
			input_schema: any
			cache_control?: { type: 'ephemeral' }
	  }>
	| undefined {
	if (!tools || tools.length === 0) return undefined

	const anthropicTools = tools.map((tool) => ({
		name: tool.function.name,
		description: tool.function.description,
		input_schema: tool.function.parameters || { type: 'object', properties: {} },
		cache_control: undefined as { type: 'ephemeral' } | undefined
	}))

	// Add cache_control to the last tool to cache all tool definitions
	if (anthropicTools.length > 0) {
		anthropicTools[anthropicTools.length - 1].cache_control = { type: 'ephemeral' }
	}

	return anthropicTools
}

export async function* convertAnthropicStreamToOpenAI(
	response: Response
): AsyncIterable<OpenAI.Chat.Completions.ChatCompletionChunk> {
	if (!response.body) {
		throw new Error('Response body is null')
	}

	const reader = response.body.getReader()
	const decoder = new TextDecoder()
	let buffer = ''
	let currentToolCall: { id: string; name: string; args: string } | null = null
	let messageId = `chatcmpl-${Date.now()}`

	try {
		while (true) {
			const { done, value } = await reader.read()
			if (done) break

			buffer += decoder.decode(value, { stream: true })
			const lines = buffer.split('\n')
			buffer = lines.pop() || ''

			for (const line of lines) {
				if (line.startsWith('data: ')) {
					const data = line.slice(6)
					if (data === '[DONE]') continue

					try {
						const event: AnthropicStreamEvent = JSON.parse(data)

						if (event.type === 'message_start') {
							yield {
								id: messageId,
								object: 'chat.completion.chunk',
								created: Math.floor(Date.now() / 1000),
								model: event.message?.model || 'claude-3-5-sonnet-20241022',
								choices: [
									{
										index: 0,
										delta: { role: 'assistant' },
										finish_reason: null
									}
								]
							}
						} else if (event.type === 'content_block_start') {
							if (event.content_block?.type === 'tool_use') {
								currentToolCall = {
									id: event.content_block.id,
									name: event.content_block.name,
									args: ''
								}
							}
						} else if (event.type === 'content_block_delta') {
							if (event.delta?.type === 'text_delta') {
								yield {
									id: messageId,
									object: 'chat.completion.chunk',
									created: Math.floor(Date.now() / 1000),
									model: 'claude-3-5-sonnet-20241022',
									choices: [
										{
											index: 0,
											delta: { content: event.delta.text },
											finish_reason: null
										}
									]
								}
							} else if (event.delta?.type === 'thinking_delta') {
								// For thinking delta, we can either include as content or skip it
								// For now, skip thinking content as it's internal to the model
							} else if (event.delta?.type === 'input_json_delta' && currentToolCall) {
								currentToolCall.args += event.delta.partial_json
							}
						} else if (event.type === 'content_block_stop') {
							if (currentToolCall) {
								// Emit tool call
								yield {
									id: messageId,
									object: 'chat.completion.chunk',
									created: Math.floor(Date.now() / 1000),
									model: 'claude-3-5-sonnet-20241022',
									choices: [
										{
											index: 0,
											delta: {
												tool_calls: [
													{
														index: 0,
														id: currentToolCall.id,
														type: 'function' as const,
														function: {
															name: currentToolCall.name,
															arguments: currentToolCall.args
														}
													}
												]
											},
											finish_reason: null
										}
									]
								}
								currentToolCall = null
							}
						} else if (event.type === 'message_delta') {
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
								model: 'claude-3-5-sonnet-20241022',
								choices: [
									{
										index: 0,
										delta: {},
										finish_reason: finishReason
									}
								]
							}
						}
					} catch (e) {
						// Skip invalid JSON
						console.warn('Failed to parse Anthropic SSE event:', data)
					}
				}
			}
		}
	} finally {
		reader.releaseLock()
	}
}
