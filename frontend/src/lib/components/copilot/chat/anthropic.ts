import { OpenAI } from 'openai'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import type {
	MessageParam,
	TextBlockParam,
	ToolUnion,
	ToolUseBlockParam,
	Tool as AnthropicTool
} from '@anthropic-ai/sdk/resources'
import { getProviderAndCompletionConfig, workspaceAIClients } from '../lib'
import type { Stream } from 'openai/streaming.mjs'
import type { Tool, ToolCallbacks } from './shared'

export async function getAnthropicCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
) {
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
			'anthropic-version': '2023-06-01'
		}
	})

	return stream
}

export async function parseAnthropicCompletion(
	completion: Stream<any>,
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	},
	messages: ChatCompletionMessageParam[],
	addedMessages: ChatCompletionMessageParam[],
	tools: Tool<any>[],
	helpers: any
): Promise<boolean> {
	// TODO: Implement Anthropic completion parsing
	return true
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
		}) as AnthropicTool.InputSchema
	}))

	// Add cache_control to the last tool to cache all tool definitions
	if (anthropicTools.length > 0) {
		anthropicTools[anthropicTools.length - 1].cache_control = { type: 'ephemeral' }
	}

	return anthropicTools
}
