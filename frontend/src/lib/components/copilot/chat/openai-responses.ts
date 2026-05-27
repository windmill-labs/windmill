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
	workspaceAIClients
} from '../lib'
import { processToolCall, type Tool, type ToolCallbacks } from './shared'
import type { ResponseStream } from 'openai/lib/responses/ResponseStream.mjs'
import type { AIProvider, AIProviderModel } from '$lib/gen'
import {
	openAIResponsesUsageToChatTokenUsage,
	type ChatTokenUsage
} from './tokenUsage'

interface ParsedCompletionResult {
	shouldContinue: boolean
	tokenUsage: ChatTokenUsage
}

type ResponseFunctionCallItem = {
	type?: unknown
	id?: unknown
	call_id?: unknown
	name?: unknown
	arguments?: unknown
}

type ToolCallMetadata = { name: string; call_id: string }

function readNonEmptyString(value: unknown): string | undefined {
	return typeof value === 'string' && value.trim().length > 0 ? value : undefined
}

function getResponseFunctionCallItem(eventOrItem: unknown): ResponseFunctionCallItem | undefined {
	const candidate = isRecord(eventOrItem)
		? (eventOrItem.item ?? eventOrItem.output_item ?? eventOrItem)
		: eventOrItem
	if (!isRecord(candidate) || candidate.type !== 'function_call') {
		return undefined
	}
	return candidate
}

function stringifyFunctionCallArguments(value: unknown): string {
	return typeof value === 'string' ? value : JSON.stringify(value ?? {})
}

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
	config: ChatCompletionCreateParams,
	provider: AIProvider
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
	const tools = 'tools' in config && config.tools ? config.tools : undefined
	if (tools && tools.length > 0) {
		responsesConfig.tools = tools.map((tool) => convertToolToResponsesTool(tool, provider))
		if (provider === 'openai_chatgpt_account') {
			responsesConfig.tool_choice = 'auto'
			responsesConfig.parallel_tool_calls = true
		}
	}
	if ('reasoning' in config && config.reasoning) {
		responsesConfig.reasoning = config.reasoning
	}

	return responsesConfig
}

function convertToolToResponsesTool(
	tool: OpenAI.Chat.Completions.ChatCompletionTool,
	provider: AIProvider
) {
	if (tool.type === 'function' && 'function' in tool) {
		return {
			type: 'function',
			name: tool.function.name,
			description: tool.function.description,
			parameters:
				provider === 'openai_chatgpt_account'
					? normalizeCodexToolParameters(tool.function.parameters as Record<string, unknown>)
					: tool.function.parameters,
			strict:
				provider === 'openai_chatgpt_account'
					? (tool.function.strict ?? true)
					: tool.function.strict ?? null
		}
	}
	return tool
}

function normalizeCodexToolParameters(schema: Record<string, unknown>): Record<string, unknown> {
	return normalizeSchemaNode(schema, true) as Record<string, unknown>
}

function normalizeSchemaNode(schema: unknown, forceRequired: boolean): unknown {
	if (!isRecord(schema)) return schema

	const normalized: Record<string, unknown> = { ...schema }

	if (Array.isArray(schema.anyOf)) {
		normalized.anyOf = schema.anyOf.map((entry) => normalizeSchemaNode(entry, forceRequired))
	}
	if (Array.isArray(schema.oneOf)) {
		normalized.oneOf = schema.oneOf.map((entry) => normalizeSchemaNode(entry, forceRequired))
	}
	if (isRecord(schema.items)) {
		normalized.items = normalizeSchemaNode(schema.items, forceRequired)
	}
	if (isRecord(schema.properties)) {
		const properties = Object.fromEntries(
			Object.entries(schema.properties).map(([key, value]) => [
				key,
				normalizeSchemaNode(value, forceRequired)
			])
		)
		const propertyKeys = Object.keys(properties)
		const originalRequired = new Set(
			Array.isArray(schema.required)
				? schema.required.filter((entry): entry is string => typeof entry === 'string')
				: []
		)

		if (forceRequired) {
			for (const key of propertyKeys) {
				if (!originalRequired.has(key) && isRecord(properties[key])) {
					properties[key] = makeSchemaNullable(properties[key])
				}
			}
		}

		normalized.properties = properties
		normalized.required = forceRequired ? propertyKeys : [...originalRequired]
		if (normalized.additionalProperties === undefined) {
			normalized.additionalProperties = false
		}
	}

	return normalized
}

function makeSchemaNullable(schema: Record<string, unknown>): Record<string, unknown> {
	if (schema.type !== undefined) {
		return { ...schema, type: appendNullToType(schema.type) }
	}
	if (Array.isArray(schema.anyOf)) {
		const hasNull = schema.anyOf.some((entry) => isRecord(entry) && entry.type === 'null')
		return hasNull ? schema : { ...schema, anyOf: [...schema.anyOf, { type: 'null' }] }
	}
	if (Array.isArray(schema.oneOf)) {
		const hasNull = schema.oneOf.some((entry) => isRecord(entry) && entry.type === 'null')
		return hasNull ? schema : { ...schema, oneOf: [...schema.oneOf, { type: 'null' }] }
	}
	return { anyOf: [schema, { type: 'null' }] }
}

function appendNullToType(type: unknown): unknown {
	if (typeof type === 'string') return type === 'null' ? 'null' : [type, 'null']
	if (Array.isArray(type)) return type.includes('null') ? type : [...type, 'null']
	return ['null']
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export async function getOpenAIResponsesCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	options?: {
		forceModelProvider?: AIProviderModel
		openaiClient?: OpenAI
	}
) {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: true,
		tools,
		forceModelProvider: options?.forceModelProvider
	})
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = convertCompletionConfigToResponsesConfig(config, provider)

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
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
): AsyncGenerator<OpenAI.Chat.Completions.ChatCompletionChunk> {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true, tools })
	const { instructions, input } = convertMessagesToResponsesInput(messages)
	const responsesConfig = convertCompletionConfigToResponsesConfig(config, provider)

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
	helpers: any,
	options?: { workspace?: string }
): Promise<ParsedCompletionResult> {
	let toolCallsToProcess: ChatCompletionMessageFunctionToolCall[] = []
	let error: OpenAIError | ResponseErrorEvent | null = null
	let textContent = ''
	let toolCallsMap: Record<string, ToolCallMetadata> = {}
	let toolCallsByCallId: Record<string, ChatCompletionMessageFunctionToolCall> = {}
	let toolCallOrder: string[] = []
	let shownToolCallIds = new Set<string>()

	// Streaming state tracking
	let currentStreamingTool:
		| { itemId: string; shouldStream: boolean; toolName: string }
		| undefined = undefined
	let accumulatedJson = ''

	const upsertToolCall = (metadata: ToolCallMetadata, args: unknown) => {
		if (!toolCallsByCallId[metadata.call_id]) {
			toolCallOrder.push(metadata.call_id)
		}
		toolCallsByCallId[metadata.call_id] = {
			id: metadata.call_id,
			type: 'function' as const,
			function: {
				name: metadata.name,
				arguments: stringifyFunctionCallArguments(args)
			}
		}
	}

	const registerToolCallItem = (
		eventOrItem: unknown,
		{ showStatus = false }: { showStatus?: boolean } = {}
	): ToolCallMetadata | undefined => {
		const item = getResponseFunctionCallItem(eventOrItem)
		if (!item) {
			return undefined
		}

		const name = readNonEmptyString(item.name)
		const toolCallId = readNonEmptyString(item.call_id) ?? readNonEmptyString(item.id)
		if (!name || !toolCallId) {
			return undefined
		}

		const responseItemId = readNonEmptyString(item.id) ?? toolCallId
		const metadata = { name, call_id: toolCallId }
		toolCallsMap[responseItemId] = metadata
		toolCallsMap[toolCallId] = metadata

		if (showStatus && !shownToolCallIds.has(toolCallId)) {
			const tool = tools.find((t) => t.def.function.name === name)
			const shouldStream = tool?.streamArguments ?? false

			accumulatedJson = ''
			currentStreamingTool = { itemId: responseItemId, shouldStream, toolName: name }
			shownToolCallIds.add(toolCallId)

			callbacks.onMessageEnd()
			callbacks.setToolStatus(`${toolCallId}`, {
				isLoading: true,
				content: `Calling ${name}...`,
				toolName: name,
				isStreamingArguments: shouldStream,
				showFade: tool?.showFade,
				showDetails: tool?.showDetails,
				autoCollapseDetails: tool?.autoCollapseDetails
			})
		}

		return metadata
	}

	// Handle text streaming
	runner.on('response.output_text.delta', (event) => {
		callbacks.onNewToken(event.delta)
		textContent += event.delta
	})

	// Handle new output items (including function calls)
	runner.on('response.output_item.added', (event) => {
		registerToolCallItem(event, { showStatus: true })
	})

	// Codex sometimes only provides the final function-call arguments on the completed item.
	runner.on('response.output_item.done', (event) => {
		const metadata = registerToolCallItem(event, { showStatus: true })
		const item = getResponseFunctionCallItem(event)
		if (!metadata || !item || item.arguments === undefined) {
			return
		}
		upsertToolCall(metadata, item.arguments)
		callbacks.setToolStatus(`${metadata.call_id}`, {
			isStreamingArguments: false
		})
	})

	// Stream function call arguments incrementally
	runner.on('response.function_call_arguments.delta', (event) => {
		const itemId = event.item_id ?? (event as { call_id?: string }).call_id
		const metadata = itemId ? toolCallsMap[itemId] : undefined
		const statusId = metadata?.call_id ?? itemId
		if (!statusId) {
			return
		}
		if (currentStreamingTool?.shouldStream && currentStreamingTool.itemId === event.item_id) {
			accumulatedJson += event.delta

			try {
				const parsed = JSON.parse(accumulatedJson)
				callbacks.setToolStatus(`${statusId}`, {
					parameters: parsed,
					isStreamingArguments: true,
					isLoading: true
				})
			} catch {
				// JSON incomplete, display as raw string
				callbacks.setToolStatus(`${statusId}`, {
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

		// Retrieve tool call metadata from map
		const itemId = event.item_id ?? (event as { call_id?: string }).call_id
		const metadata = itemId ? toolCallsMap[itemId] : undefined
		if (!metadata) {
			console.error('Missing tool call metadata for:', itemId)
			return
		}
		const statusId = metadata.call_id

		// Convert to OpenAI format for compatibility with existing tool processing
		upsertToolCall(metadata, event.arguments)
		callbacks.setToolStatus(`${statusId}`, {
			isStreamingArguments: false
		})
	})

	runner.on('response.completed', (event) => {
		const output = (event as { response?: { output?: unknown[] } }).response?.output
		if (!Array.isArray(output)) {
			return
		}
		for (const item of output) {
			const metadata = registerToolCallItem(item, { showStatus: true })
			const functionCall = getResponseFunctionCallItem(item)
			if (!metadata || !functionCall || functionCall.arguments === undefined) {
				continue
			}
			upsertToolCall(metadata, functionCall.arguments)
			callbacks.setToolStatus(`${metadata.call_id}`, {
				isStreamingArguments: false
			})
		}
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
	toolCallsToProcess = toolCallOrder.map((callId) => toolCallsByCallId[callId]).filter(Boolean)

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
		return { shouldContinue: true, tokenUsage }
	}

	return { shouldContinue: false, tokenUsage }
}

export async function getNonStreamingOpenAIResponsesCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	testOptions?: {
		apiKey?: string
		workspace?: string
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
	const responsesConfig = convertCompletionConfigToResponsesConfig(config, provider)

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
		? createOpenAIProxyClient(getAiProxyBaseURL())
		: testOptions?.workspace
			? workspaceAIClients.createOpenaiClient(testOptions.workspace)
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
