import OpenAI, { OpenAIError } from 'openai'
import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionCreateParams
} from 'openai/resources/index.mjs'
import type { ResponseErrorEvent } from 'openai/resources/responses/responses.mjs'
import { getProviderAndCompletionConfig, workspaceAIClients } from '../lib'
import { processToolCall, type Tool, type ToolCallbacks } from './shared'
import type { ResponseStream } from 'openai/lib/responses/ResponseStream.mjs'
import { OpenAPI } from '$lib/gen'
import type { AIProviderModel } from '$lib/gen'

// Conversion utilities for Responses API
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
				content: m.content
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

	// Keep other relevant fields
	if (config.temperature !== undefined) {
		responsesConfig.temperature = config.temperature
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
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
) {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true, tools })
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = convertCompletionConfigToResponsesConfig(config)

	const openaiClient = workspaceAIClients.getOpenaiClient()

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

	return runner
}

// Wrapper that converts ResponseStream to ChatCompletionChunk format for lib.ts usage
export async function* getOpenAIResponsesCompletionStream(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
): AsyncGenerator<OpenAI.Chat.Completions.ChatCompletionChunk> {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true, tools })
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = convertCompletionConfigToResponsesConfig(config)

	const openaiClient = workspaceAIClients.getOpenaiClient()

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
	helpers: any
): Promise<boolean> {
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

	// Handle new output items (including function calls)
	runner.on('response.output_item.added', (event) => {
		const item = event.item
		if (item.type === 'function_call' && item.id) {
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
				content: `Calling ${item.name}...`,
				toolName: item.name,
				isStreamingArguments: shouldStream,
				showFade: tool?.showFade,
				showDetails: tool?.showDetails
			})
		}
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

export async function getNonStreamingOpenAIResponsesCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	testOptions?: {
		apiKey?: string
		resourcePath?: string
		forceModelProvider: AIProviderModel
	}
): Promise<string> {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: false,
		forceModelProvider: testOptions?.forceModelProvider
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

	if (testOptions?.resourcePath) {
		fetchOptions.headers = {
			...fetchOptions.headers,
			'X-Resource-Path': testOptions.resourcePath
		}
	} else if (testOptions?.apiKey) {
		fetchOptions.headers = {
			...fetchOptions.headers,
			'X-API-Key': testOptions.apiKey
		}
	}

	const openaiClient = testOptions?.apiKey
		? new OpenAI({
				baseURL: `${location.origin}${OpenAPI.BASE}/ai/proxy`,
				apiKey: 'fake-key',
				defaultHeaders: {
					Authorization: '' // a non empty string will be unable to access Windmill backend proxy
				},
				dangerouslyAllowBrowser: true
			})
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
