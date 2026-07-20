import OpenAI, { OpenAIError } from 'openai'
import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionCreateParams
} from 'openai/resources/index.mjs'
import type { ResponseErrorEvent } from 'openai/resources/responses/responses.mjs'
import {
	createOpenAIProxyClient,
	getAiProxyBaseURL,
	getProviderAndCompletionConfig,
	providerSupportsWebSearch,
	workspaceAIClients
} from '../lib'
import { applyReasoningToConfig } from '../reasoningRegistry'
import { appendPendingToolImages, processToolCall, type Tool, type ToolCallbacks } from './shared'
import type { ResponseStream } from 'openai/lib/responses/ResponseStream.mjs'
import type { AIProviderModel } from '$lib/gen'
import { openAIResponsesUsageToChatTokenUsage, type ChatTokenUsage } from './tokenUsage'

interface ParsedCompletionResult {
	shouldContinue: boolean
	tokenUsage: ChatTokenUsage
}

type WebSearchStatus = 'in_progress' | 'searching' | 'completed' | 'failed'

const openAIWebSearchToolId = (itemId: string) => `openai_web_search:${itemId}`

function setOpenAIWebSearchStatus(
	callbacks: ToolCallbacks & { onMessageEnd: () => void },
	itemId: string,
	status: WebSearchStatus
) {
	const isLoading = status === 'in_progress' || status === 'searching'
	const failed = status === 'failed'
	callbacks.onMessageEnd()
	callbacks.setToolStatus(openAIWebSearchToolId(itemId), {
		content: failed ? 'Web search failed' : isLoading ? 'Searching the web...' : 'Searched the web',
		error: failed ? 'Web search failed' : undefined,
		isLoading,
		isStreamingArguments: false,
		needsConfirmation: false,
		toolName: 'web_search',
		showDetails: false,
		autoCollapseDetails: true
	})
}

// Conversion utilities for Responses API

/**
 * Translate Chat-Completions message content to Responses-native content. Strings
 * pass through; a content-part array maps text→input_text and image_url→input_image
 * (Responses takes image_url as a plain string, not the {url} object).
 */
export function toResponsesContent(content: unknown): unknown {
	if (!Array.isArray(content)) return content
	return content.map((part) => {
		if (part?.type === 'text') return { type: 'input_text', text: part.text }
		if (part?.type === 'image_url' && part.image_url?.url) {
			return { type: 'input_image', image_url: part.image_url.url }
		}
		return part
	})
}

function convertMessagesToResponsesInput(messages: ChatCompletionMessageParam[]): {
	instructions?: string
	input: Array<any>
} {
	const systemMessage = messages.find((m) => m.role === 'system')
	const nonSystemMessages = messages.filter((m) => m.role !== 'system')

	const input: Array<any> = []

	for (const m of nonSystemMessages) {
		// Handle assistant messages with tool calls
		if (m.role === 'assistant' && 'tool_calls' in m && m.tool_calls) {
			// First, add the assistant's text content if it exists
			if ('content' in m && m.content) {
				input.push({
					type: 'message' as const,
					role: 'assistant',
					content: m.content
				})
			}

			// Then add tool calls
			for (const toolCall of m.tool_calls) {
				if (toolCall.type === 'function') {
					// Validate that required fields exist
					if (!toolCall.function.name || toolCall.function.name.trim() === '') {
						console.warn('Skipping tool call with empty name:', toolCall)
						continue
					}

					input.push({
						type: 'function_call',
						call_id: toolCall.id,
						name: toolCall.function.name,
						arguments: toolCall.function.arguments
					})
				}
			}
		}
		// Handle tool result messages
		else if (m.role === 'tool' && 'content' in m && 'tool_call_id' in m) {
			input.push({
				type: 'function_call_output',
				call_id: m.tool_call_id,
				output: typeof m.content === 'string' ? m.content : JSON.stringify(m.content)
			})
		}
		// Handle regular messages
		else if ('content' in m && m.content !== null && m.content !== undefined) {
			input.push({
				type: 'message' as const,
				role: m.role === 'developer' ? 'developer' : m.role === 'assistant' ? 'assistant' : 'user',
				content: toResponsesContent(m.content)
			})
		}
	}

	return {
		instructions:
			systemMessage && 'content' in systemMessage
				? typeof systemMessage.content === 'string'
					? systemMessage.content
					: JSON.stringify(systemMessage.content)
				: undefined,
		input
	}
}

function convertCompletionConfigToResponsesConfig(
	config: ChatCompletionCreateParams
): Record<string, any> {
	const responsesConfig: Record<string, any> = {
		model: config.model
	}

	// Map max_tokens or max_completion_tokens to max_output_tokens
	if ('max_completion_tokens' in config && config.max_completion_tokens) {
		responsesConfig.max_output_tokens = config.max_completion_tokens
	} else if ('max_tokens' in config && config.max_tokens) {
		responsesConfig.max_output_tokens = config.max_tokens
	}

	if ('tools' in config && config.tools && config.tools.length > 0) {
		responsesConfig.tools = config.tools.map((tool) => {
			if (tool.type === 'function' && 'function' in tool) {
				// Convert from Completions format to Responses format
				return {
					type: 'function',
					name: tool.function.name,
					description: tool.function.description,
					parameters: tool.function.parameters,
					strict: tool.function.strict ?? null
				}
			}
			// Pass through other tool types unchanged
			return tool
		})
	}

	return responsesConfig
}

export async function getOpenAIResponsesCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	options?: {
		forceModelProvider?: AIProviderModel
		openaiClient?: OpenAI
		webSearch?: boolean
		reasoningEffort?: string
		reasoningSummary?: boolean
	}
) {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: true,
		tools,
		forceModelProvider: options?.forceModelProvider
	})
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = applyReasoningToConfig(
		convertCompletionConfigToResponsesConfig(config),
		'responses',
		options?.reasoningEffort
	)

	// Reasoning summaries make the model's thinking renderable in the chat, but
	// OpenAI rejects the request (400 on reasoning.summary) for organizations
	// that haven't completed verification — callers opt in and fall back.
	if (options?.reasoningSummary && responsesConfig.reasoning) {
		responsesConfig.reasoning = { ...responsesConfig.reasoning, summary: 'auto' }
	}

	// Enable OpenAI's built-in web search tool. The proxy forwards the body
	// verbatim, so this reaches OpenAI as a native server-side tool.
	if (options?.webSearch && providerSupportsWebSearch(provider)) {
		responsesConfig.tools = [...(responsesConfig.tools ?? []), { type: 'web_search' }]
	}

	const client = options?.openaiClient ?? workspaceAIClients.getOpenaiClient()

	const runner = client.responses.stream(
		{
			...responsesConfig,
			input,
			...(instructions ? { instructions } : {})
		},
		{
			signal: abortController.signal,
			headers: {
				'X-Provider': provider
			}
		}
	)

	return runner
}

// Wrapper that converts ResponseStream to ChatCompletionChunk format for lib.ts usage
export async function* getOpenAIResponsesCompletionStream(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	options?: {
		forceModelProvider?: AIProviderModel
		openaiClient?: OpenAI
		reasoningEffort?: string
	}
): AsyncGenerator<OpenAI.Chat.Completions.ChatCompletionChunk> {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: true,
		tools,
		forceModelProvider: options?.forceModelProvider
	})
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = applyReasoningToConfig(
		convertCompletionConfigToResponsesConfig(config),
		'responses',
		options?.reasoningEffort
	)

	const openaiClient = options?.openaiClient ?? workspaceAIClients.getOpenaiClient()

	const runner = openaiClient.responses.stream(
		{
			...responsesConfig,
			input,
			...(instructions ? { instructions } : {})
		},
		{
			signal: abortController.signal,
			headers: {
				'X-Provider': provider
			}
		}
	)

	// Convert ResponseStream events to ChatCompletionChunk format
	for await (const event of runner) {
		if (event.type === 'response.output_text.delta') {
			// Yield text chunks in ChatCompletionChunk format
			yield {
				id: 'chatcmpl-' + Date.now(),
				object: 'chat.completion.chunk',
				created: Date.now(),
				model: responsesConfig.model,
				choices: [
					{
						index: 0,
						delta: {
							content: event.delta || ''
						},
						finish_reason: null
					}
				]
			} as OpenAI.Chat.Completions.ChatCompletionChunk
		}
	}
}

export async function parseOpenAIResponsesCompletion(
	runner: ResponseStream,
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	},
	messages: ChatCompletionMessageParam[],
	addedMessages: ChatCompletionMessageParam[],
	tools: Tool<any>[],
	helpers: any,
	options?: { workspace?: string }
): Promise<ParsedCompletionResult> {
	let toolCallsToProcess: ChatCompletionMessageFunctionToolCall[] = []
	let error: OpenAIError | ResponseErrorEvent | null = null
	let textContent = ''
	let toolCallsMap: Record<string, { name: string; call_id: string }> = {}

	// Streaming state tracking
	let currentStreamingTool:
		| { itemId: string; shouldStream: boolean; toolName: string }
		| undefined = undefined
	let accumulatedJson = ''

	// Handle text streaming
	runner.on('response.output_text.delta', (event) => {
		callbacks.onNewToken(event.delta)
		textContent += event.delta
	})

	// Stream the reasoning summary (present when the request asked for
	// reasoning.summary) into the thinking display. Summaries arrive as
	// separate parts; join them as paragraphs.
	let reasoningSummaryParts = 0
	runner.on('response.reasoning_summary_part.added', () => {
		reasoningSummaryParts++
		if (reasoningSummaryParts > 1) {
			callbacks.onReasoningDelta?.('\n\n')
		}
	})
	runner.on('response.reasoning_summary_text.delta', (event) => {
		callbacks.onReasoningDelta?.(event.delta)
	})

	// Handle new output items (including function calls)
	runner.on('response.output_item.added', (event) => {
		const item = event.item
		if (item.type === 'reasoning') {
			// Reasoning models (GPT/o-series) emit a reasoning item but no chain-of-thought
			// text; surface a live "Thinking" indicator so the user can see it reason.
			callbacks.onReasoningStart?.()
		} else if (item.type === 'function_call' && item.id) {
			const tool = tools.find((t) => t.def.function.name === item.name)
			const shouldStream = tool?.streamArguments ?? false

			toolCallsMap[item.id] = {
				name: item.name,
				call_id: item.call_id
			}

			// Reset streaming state for new tool
			accumulatedJson = ''
			currentStreamingTool = { itemId: item.id, shouldStream, toolName: item.name }

			// Show temporary loading state for the tool call
			callbacks.onMessageEnd()
			callbacks.setToolStatus(`${item.id}`, {
				isLoading: true,
				content: tool?.streamingLabel ?? `Calling ${item.name}...`,
				toolName: item.name,
				isStreamingArguments: shouldStream,
				showFade: tool?.showFade,
				showDetails: tool?.showDetails,
				autoCollapseDetails: tool?.autoCollapseDetails
			})
		} else if (item.type === 'web_search_call' && item.id) {
			setOpenAIWebSearchStatus(callbacks, item.id, item.status)
		}
	})

	runner.on('response.web_search_call.in_progress', (event) => {
		setOpenAIWebSearchStatus(callbacks, event.item_id, 'in_progress')
	})

	runner.on('response.web_search_call.searching', (event) => {
		setOpenAIWebSearchStatus(callbacks, event.item_id, 'searching')
	})

	runner.on('response.web_search_call.completed', (event) => {
		setOpenAIWebSearchStatus(callbacks, event.item_id, 'completed')
	})

	// Stream function call arguments incrementally
	runner.on('response.function_call_arguments.delta', (event) => {
		if (currentStreamingTool?.shouldStream && currentStreamingTool.itemId === event.item_id) {
			accumulatedJson += event.delta

			try {
				const parsed = JSON.parse(accumulatedJson)
				callbacks.setToolStatus(`${event.item_id}`, {
					parameters: parsed,
					isStreamingArguments: true,
					isLoading: true
				})
			} catch {
				// JSON incomplete, display as raw string
				callbacks.setToolStatus(`${event.item_id}`, {
					parameters: accumulatedJson,
					isStreamingArguments: true,
					isLoading: true
				})
			}
		}
	})

	// Handle function call arguments done
	runner.on('response.function_call_arguments.done', (event) => {
		// Clear streaming state
		currentStreamingTool = undefined
		callbacks.setToolStatus(`${event.item_id}`, {
			isStreamingArguments: false
		})

		// Retrieve tool call metadata from map
		const metadata = toolCallsMap[event.item_id]
		if (!metadata) {
			console.error('Missing tool call metadata for:', event.item_id)
			return
		}

		// Convert to OpenAI format for compatibility with existing tool processing
		toolCallsToProcess.push({
			id: event.item_id,
			type: 'function' as const,
			function: {
				name: metadata.name,
				arguments: event.arguments
			}
		})
	})

	// Handle errors
	runner.on('error', (err: OpenAIError | ResponseErrorEvent) => {
		currentStreamingTool = undefined
		console.error('OpenAI Responses stream error:', err)
		error = err
	})

	// Wait for completion
	await runner.done()

	// Add text message if we got any text
	if (textContent) {
		const assistantMessage = { role: 'assistant' as const, content: textContent }
		messages.push(assistantMessage)
		addedMessages.push(assistantMessage)
		callbacks.onMessageEnd()
	}

	if (error) {
		throw error
	}

	const finalResponse = await runner.finalResponse()
	const tokenUsage = openAIResponsesUsageToChatTokenUsage(finalResponse.usage)

	for (const item of finalResponse.output ?? []) {
		if (item.type === 'web_search_call') {
			setOpenAIWebSearchStatus(callbacks, item.id, item.status)
		}
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
				toolCallbacks: callbacks,
				workspace: options?.workspace
			})
			messages.push(messageToAdd)
			addedMessages.push(messageToAdd)
		}
		appendPendingToolImages(messages, addedMessages, callbacks)
		return { shouldContinue: true, tokenUsage }
	}

	return { shouldContinue: false, tokenUsage }
}

export async function getNonStreamingOpenAIResponsesCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	options?: {
		apiKey?: string
		workspace?: string
		resourcePath?: string
		forceModelProvider?: AIProviderModel
		maxTokensCap?: number
	}
): Promise<string> {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: false,
		forceModelProvider: options?.forceModelProvider,
		maxTokensCap: options?.maxTokensCap
	})

	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = convertCompletionConfigToResponsesConfig(config)

	const fetchOptions: {
		signal: AbortSignal
		headers: Record<string, string>
	} = {
		signal: abortController.signal,
		headers: {
			'X-Provider': provider
		}
	}

	if (options?.resourcePath) {
		fetchOptions.headers = {
			...fetchOptions.headers,
			'X-Resource-Path': options.resourcePath
		}
	} else if (options?.apiKey) {
		fetchOptions.headers = {
			...fetchOptions.headers,
			'X-API-Key': options.apiKey
		}
	}

	const openaiClient = options?.apiKey
		? createOpenAIProxyClient(getAiProxyBaseURL())
		: options?.workspace
			? workspaceAIClients.createOpenaiClient(options.workspace)
			: workspaceAIClients.getOpenaiClient()

	const response = await openaiClient.responses.create(
		{
			...responsesConfig,
			input,
			...(instructions ? { instructions } : {})
		},
		fetchOptions
	)

	return response.output_text || ''
}
