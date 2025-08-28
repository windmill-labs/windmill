import type { AIProvider, AIProviderModel } from '$lib/gen'
import {
	getCurrentModel,
	workspaceStore,
	type DBSchema,
	type GraphqlSchema,
	type SQLSchema
} from '$lib/stores'
import { buildClientSchema, printSchema } from 'graphql'
import { OpenAI } from 'openai'
import type {
	ChatCompletionCreateParams,
	ChatCompletionCreateParamsNonStreaming,
	ChatCompletionCreateParamsStreaming,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'
import { get, type Writable } from 'svelte/store'
import { OpenAPI, ResourceService, type Script } from '../../gen'
import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import { formatResourceTypes } from './utils'
import { z } from 'zod'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

const OPENAI_MODELS = [
	'gpt-5',
	'gpt-5-mini',
	'gpt-5-nano',
	'gpt-4o',
	'gpt-4o-mini',
	'o4-mini',
	'o3',
	'o3-mini'
]

// need at least one model for each provider except customai
export const AI_DEFAULT_MODELS: Record<AIProvider, string[]> = {
	openai: OPENAI_MODELS,
	azure_openai: OPENAI_MODELS,
	anthropic: ['claude-sonnet-4-0', 'claude-sonnet-4-0/thinking', 'claude-3-5-haiku-latest'],
	mistral: ['codestral-latest'],
	deepseek: ['deepseek-chat', 'deepseek-reasoner'],
	googleai: ['gemini-2.0-flash', 'gemini-1.5-flash', 'gemini-1.5-pro'],
	groq: ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant'],
	openrouter: ['meta-llama/llama-3.2-3b-instruct:free'],
	togetherai: ['meta-llama/Llama-3.3-70B-Instruct-Turbo'],
	customai: []
}

export interface ModelResponse {
	id: string
	object: string
	created: number
	owned_by: string
	lifecycle_status: string
	capabilities: {
		completion: boolean
		chat_completion: boolean
	}
}

export async function fetchAvailableModels(
	resourcePath: string,
	workspace: string,
	provider: AIProvider
): Promise<string[]> {
	const models = await fetch(`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/models`, {
		headers: {
			'X-Resource-Path': resourcePath,
			'X-Provider': provider,
			...(provider === 'anthropic' ? { 'anthropic-version': '2023-06-01' } : {})
		}
	})
	if (!models.ok) {
		console.error('Failed to fetch models for provider', provider, models)
		throw new Error(`Failed to fetch models for provider ${provider}`)
	}
	const data = (await models.json()) as { data: ModelResponse[] }
	if (data.data.length > 0) {
		switch (provider) {
			case 'openai':
				return data.data
					.filter(
						(m) => m.id.startsWith('gpt-') || m.id.startsWith('o') || m.id.startsWith('codex')
					)
					.map((m) => m.id)
			case 'azure_openai':
				return data.data
					.filter(
						(m) =>
							(m.id.startsWith('gpt-') || m.id.startsWith('o') || m.id.startsWith('codex')) &&
							m.lifecycle_status !== 'deprecated' &&
							(m.capabilities.completion || m.capabilities.chat_completion)
					)
					.map((m) => m.id)
			case 'googleai':
				return data.data.map((m) => m.id.split('/')[1])
			default:
				return data.data.map((m) => m.id)
		}
	}

	return data?.data.map((m) => m.id) ?? []
}

function getModelMaxTokens(provider: AIProvider, model: string) {
	if (model.startsWith('gpt-5')) {
		return 128000
	} else if ((provider === 'azure_openai' || provider === 'openai') && model.startsWith('o')) {
		return 100000
	} else if (model.startsWith('gpt-4.1')) {
		return 32768
	} else if (model.startsWith('gpt-4o') || model.startsWith('codestral')) {
		return 16384
	} else if (model.startsWith('gpt-4-turbo') || model.startsWith('gpt-3.5')) {
		return 4096
	}
	return 8192
}

export function getModelContextWindow(model: string) {
	if (model.startsWith('gpt-4.1') || model.startsWith('gemini')) {
		return 1000000
	} else if (model.startsWith('gpt-5')) {
		return 400000
	} else if (model.startsWith('gpt-4o') || model.startsWith('llama-3.3')) {
		return 128000
	} else if (model.startsWith('claude') || model.startsWith('o4-mini') || model.startsWith('o3')) {
		return 200000
	} else if (model.startsWith('codestral')) {
		return 32000
	} else {
		return 128000
	}
}

function getModelSpecificConfig(
	modelProvider: AIProviderModel,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
) {
	if (
		(modelProvider.provider === 'openai' || modelProvider.provider === 'azure_openai') &&
		(modelProvider.model.startsWith('o') || modelProvider.model.startsWith('gpt-5'))
	) {
		return {
			model: modelProvider.model,
			...(tools && tools.length > 0 ? { tools } : {}),
			max_completion_tokens: getModelMaxTokens(modelProvider.provider, modelProvider.model)
		}
	} else {
		return {
			...(modelProvider.model.endsWith('/thinking')
				? {
						thinking: {
							type: 'enabled',
							budget_tokens: 1024
						},
						model: modelProvider.model.slice(0, -9)
					}
				: {
						model: modelProvider.model,
						temperature: 0
					}),
			...(tools && tools.length > 0 ? { tools } : {}),
			max_tokens: getModelMaxTokens(modelProvider.provider, modelProvider.model)
		}
	}
}

function prepareMessages(aiProvider: AIProvider, messages: ChatCompletionMessageParam[]) {
	switch (aiProvider) {
		case 'googleai':
			// system messages are not supported by gemini
			const systemMessage = messages.find((m) => m.role === 'system')
			if (systemMessage) {
				messages.shift()
				const startMessages: ChatCompletionMessageParam[] = [
					{
						role: 'user',
						content: 'System prompt: ' + (systemMessage.content as string)
					},
					{
						role: 'assistant',
						content: 'Understood'
					}
				]
				messages = [...startMessages, ...messages]
			}
			return messages
		default:
			return messages
	}
}

const DEFAULT_COMPLETION_CONFIG: ChatCompletionCreateParams = {
	model: '',
	seed: 42,
	messages: []
}

export const PROVIDER_COMPLETION_CONFIG_MAP: Record<AIProvider, ChatCompletionCreateParams> = {
	openai: DEFAULT_COMPLETION_CONFIG,
	azure_openai: DEFAULT_COMPLETION_CONFIG,
	groq: DEFAULT_COMPLETION_CONFIG,
	openrouter: DEFAULT_COMPLETION_CONFIG,
	togetherai: DEFAULT_COMPLETION_CONFIG,
	deepseek: DEFAULT_COMPLETION_CONFIG,
	customai: DEFAULT_COMPLETION_CONFIG,
	googleai: {
		...DEFAULT_COMPLETION_CONFIG,
		seed: undefined // not supported by gemini
	} as ChatCompletionCreateParams,
	mistral: {
		...DEFAULT_COMPLETION_CONFIG,
		seed: undefined
	},
	anthropic: DEFAULT_COMPLETION_CONFIG
} as const

class WorkspacedAIClients {
	private openaiClient: OpenAI | undefined

	init(workspace: string) {
		this.initOpenai(workspace)
	}

	private getBaseURL(workspace: string) {
		return `${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy`
	}

	private initOpenai(workspace: string) {
		const baseURL = this.getBaseURL(workspace)
		this.openaiClient = new OpenAI({
			baseURL,
			apiKey: 'fake-key',
			defaultHeaders: {
				Authorization: '' // a non empty string will be unable to access Windmill backend proxy
			},
			dangerouslyAllowBrowser: true
		})
	}

	getOpenaiClient() {
		if (!this.openaiClient) {
			throw new Error('OpenAI not initialized')
		}
		return this.openaiClient
	}
}

export const workspaceAIClients = new WorkspacedAIClients()

export async function testKey({
	apiKey,
	resourcePath,
	model,
	abortController,
	messages,
	aiProvider
}: {
	apiKey?: string
	resourcePath?: string
	model: string | undefined
	messages: ChatCompletionMessageParam[]
	abortController: AbortController
	aiProvider: AIProvider
}) {
	if (!apiKey && !resourcePath) {
		throw new Error('API key or resource path is required')
	}
	const modelToTest = model ?? AI_DEFAULT_MODELS[aiProvider][0]

	if (!modelToTest) {
		throw new Error('Missing a model to test')
	}

	await getNonStreamingCompletion(messages, abortController, {
		apiKey,
		resourcePath,
		forceModelProvider: {
			model: modelToTest,
			provider: aiProvider
		}
	})
}

interface BaseOptions {
	language: Script['language'] | 'frontend' | 'transformer'
	dbSchema: DBSchema | undefined
	workspace: string
}

interface ScriptGenerationOptions extends BaseOptions {
	description: string
	type: 'gen'
}

interface EditScriptOptions extends BaseOptions {
	description: string
	code: string
	type: 'edit'
}

interface FixScriptOpions extends BaseOptions {
	code: string
	error: string
	type: 'fix'
}

type CopilotOptions = ScriptGenerationOptions | EditScriptOptions | FixScriptOpions

async function getResourceTypes(scriptOptions: CopilotOptions) {
	const elems =
		scriptOptions.type === 'gen' || scriptOptions.type === 'edit' ? [scriptOptions.description] : []

	if (scriptOptions.type === 'edit' || scriptOptions.type === 'fix') {
		const { code } = scriptOptions

		const mainSig =
			scriptOptions.language === 'python3'
				? code.match(/def main\((.*?)\)/s)
				: code.match(/function main\((.*?)\)/s)

		if (mainSig) {
			elems.push(mainSig[1])
		}

		const matches = code.matchAll(/^(?:type|class) ([a-zA-Z0-9_]+)/gm)

		for (const match of matches) {
			elems.push(match[1])
		}
	}

	const resourceTypes = await ResourceService.queryResourceTypes({
		workspace: scriptOptions.workspace,
		text: elems.join(';'),
		limit: 3
	})

	return resourceTypes
}

export async function addResourceTypes(scriptOptions: CopilotOptions, prompt: string) {
	if (['deno', 'bun', 'nativets', 'python3', 'php'].includes(scriptOptions.language)) {
		const resourceTypes = await getResourceTypes(scriptOptions)
		const resourceTypesText = formatResourceTypes(
			resourceTypes,
			['deno', 'bun', 'nativets'].includes(scriptOptions.language)
				? 'typescript'
				: (scriptOptions.language as 'python3' | 'php')
		)
		prompt = prompt.replace('{resourceTypes}', resourceTypesText)
	}
	return prompt
}

export const MAX_SCHEMA_LENGTH = 100000 * 3.5

export function addThousandsSeparator(n: number) {
	return n.toFixed().replace(/\B(?=(\d{3})+(?!\d))/g, "'")
}

export function stringifySchema(
	dbSchema: Omit<SQLSchema, 'stringified'> | Omit<GraphqlSchema, 'stringified'>
) {
	const { schema, lang } = dbSchema
	if (lang === 'graphql') {
		let graphqlSchema = printSchema(buildClientSchema(schema))
		return graphqlSchema
	} else {
		let smallerSchema: {
			[schemaKey: string]: {
				[tableKey: string]: Array<[string, string, boolean, string?]>
			}
		} = {}
		for (const schemaKey in schema) {
			smallerSchema[schemaKey] = {}
			for (const tableKey in schema[schemaKey]) {
				smallerSchema[schemaKey][tableKey] = []
				for (const colKey in schema[schemaKey][tableKey]) {
					const col = schema[schemaKey][tableKey][colKey]
					const p: [string, string, boolean, string?] = [colKey, col.type, col.required]
					if (col.default) {
						p.push(col.default)
					}
					smallerSchema[schemaKey][tableKey].push(p)
				}
			}
		}

		let finalSchema: typeof smallerSchema | (typeof smallerSchema)['schemaKey'] = smallerSchema
		if (dbSchema.publicOnly) {
			finalSchema =
				smallerSchema.public || smallerSchema.PUBLIC || smallerSchema.dbo || smallerSchema
		} else if (lang === 'mysql' && Object.keys(smallerSchema).length === 1) {
			finalSchema = smallerSchema[Object.keys(smallerSchema)[0]]
		}
		return JSON.stringify(finalSchema)
	}
}

function addDBSChema(scriptOptions: CopilotOptions, prompt: string) {
	const { dbSchema, language } = scriptOptions
	if (
		dbSchema &&
		['postgresql', 'mysql', 'snowflake', 'bigquery', 'mssql', 'graphql', 'oracledb'].includes(
			language
		) && // make sure we are using a SQL/query language
		language === dbSchema.lang // make sure we are using the same language as the schema
	) {
		let { stringified } = dbSchema
		if (dbSchema.lang === 'graphql') {
			if (stringified.length > MAX_SCHEMA_LENGTH) {
				stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
			}
			prompt = prompt + '\nHere is the GraphQL schema: <schema>\n' + stringified + '\n</schema>'
		} else {
			if (stringified.length > MAX_SCHEMA_LENGTH) {
				stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
			}
			prompt =
				prompt +
				"\nHere's the database schema, each column is in the format [name, type, required, default?]: <dbschema>\n" +
				stringified +
				'\n</dbschema>'
		}
	}
	return prompt
}

async function getPrompts(scriptOptions: CopilotOptions) {
	const promptsConfig = PROMPTS_CONFIGS[scriptOptions.type]
	let prompt = promptsConfig.prompts[scriptOptions.language].prompt
	if (scriptOptions.type !== 'fix') {
		prompt = prompt.replace('{description}', scriptOptions.description)
	}

	if (scriptOptions.type !== 'gen') {
		prompt = prompt.replace('{code}', scriptOptions.code)
	}

	if (scriptOptions.type === 'fix') {
		if (scriptOptions.language === 'frontend') {
			throw new Error('Fixing frontend code is not supported')
		}
		prompt = prompt.replace('{error}', scriptOptions.error)
	}

	prompt = await addResourceTypes(scriptOptions, prompt)

	prompt = addDBSChema(scriptOptions, prompt)

	return { prompt, systemPrompt: promptsConfig.system }
}

const PROMPTS_CONFIGS = {
	fix: FIX_CONFIG,
	edit: EDIT_CONFIG,
	gen: GEN_CONFIG
}

function getProviderAndCompletionConfig<K extends boolean>({
	messages,
	stream,
	tools,
	forceModelProvider
}: {
	messages: ChatCompletionMessageParam[]
	stream: K
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
	forceModelProvider?: AIProviderModel
}): {
	provider: AIProvider
	config: K extends true
		? ChatCompletionCreateParamsStreaming
		: ChatCompletionCreateParamsNonStreaming
} {
	const modelProvider = forceModelProvider ?? getCurrentModel()
	const providerConfig = PROVIDER_COMPLETION_CONFIG_MAP[modelProvider.provider]
	const processedMessages = prepareMessages(modelProvider.provider, messages)
	return {
		provider: modelProvider.provider,
		config: {
			...providerConfig,
			...getModelSpecificConfig(modelProvider, tools),
			messages: processedMessages,
			stream
		} as any
	}
}

export async function getNonStreamingCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	testOptions?: {
		apiKey?: string // testing API KEY using the global ai proxy
		resourcePath?: string // testing resource path passed as a header to the backend proxy
		forceModelProvider: AIProviderModel
	}
) {
	let response: string | undefined = ''
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: false,
		forceModelProvider: testOptions?.forceModelProvider
	})

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
		if (provider === 'customai') {
			throw new Error('Cannot test API key for Custom AI, only resource path is supported')
		}

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

	const completion = await openaiClient.chat.completions.create(config, fetchOptions)
	response = completion.choices?.[0]?.message.content || ''
	return response
}

const mistralFimResponseSchema = z.object({
	choices: z.array(
		z.object({
			message: z.object({
				content: z.string().optional()
			}),
			finish_reason: z.string()
		})
	)
})

export const FIM_MAX_TOKENS = 256
const FIM_MAX_LINES = 8
export async function getFimCompletion(
	prompt: string,
	suffix: string,
	providerModel: AIProviderModel,
	abortController: AbortController
): Promise<string | undefined> {
	const fetchOptions: {
		signal: AbortSignal
		headers: Record<string, string>
	} = {
		signal: abortController.signal,
		headers: {
			'X-Provider': providerModel.provider
		}
	}

	const workspace = get(workspaceStore)

	const response = await fetch(
		`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/fim/completions`,
		{
			method: 'POST',
			body: JSON.stringify({
				model: providerModel.model,
				temperature: 0,
				prompt,
				suffix,
				stop: ['\n\n'],
				max_tokens: FIM_MAX_TOKENS
			}),
			...fetchOptions
		}
	)

	const body = await response.json()
	const parsedBody = mistralFimResponseSchema.parse(body)

	const choice = parsedBody.choices[0]

	if (choice && choice.message.content !== undefined) {
		let lines = choice.message.content.split('\n')

		// If finish_reason is 'length', remove the last line
		if (choice.finish_reason === 'length') {
			if (lines.length > 1) {
				lines = lines.slice(0, -1)
			} else {
				lines = []
			}
		}

		lines = lines.slice(0, FIM_MAX_LINES)

		return lines.join('\n')
	} else {
		return undefined
	}
}

export async function getCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
) {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true, tools })

	// If using Anthropic, handle format conversion and direct API call
	if (provider === 'anthropic') {
		const { system, messages: anthropicMessages } = convertOpenAIToAnthropicMessages(messages)
		const anthropicTools = convertOpenAIToolsToAnthropic(tools)

		const anthropicRequest: AnthropicRequest = {
			model: config.model,
			max_tokens: config.max_tokens || 4096,
			messages: anthropicMessages,
			stream: true,
			...(system && { system }),
			...(anthropicTools && { tools: anthropicTools }),
			...(typeof config.temperature === 'number' && { temperature: config.temperature })
		}

		const workspace = get(workspaceStore)
		const response = await fetch(
			`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/messages`,
			{
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					'X-Provider': provider,
					'anthropic-version': '2023-06-01'
				},
				body: JSON.stringify(anthropicRequest),
				signal: abortController.signal
			}
		)

		if (!response.ok) {
			throw new Error(`Anthropic API request failed: ${response.status} ${response.statusText}`)
		}

		return convertAnthropicStreamToOpenAI(response)
	}

	// For other providers, use the existing OpenAI client
	const openaiClient = workspaceAIClients.getOpenaiClient()
	const completion = await openaiClient.chat.completions.create(config, {
		signal: abortController.signal,
		headers: {
			'X-Provider': provider
		}
	})
	return completion
}

export function getResponseFromEvent(part: OpenAI.Chat.Completions.ChatCompletionChunk): string {
	return part.choices?.[0]?.delta?.content || ''
}

// Anthropic API types and conversion functions
interface AnthropicMessage {
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

interface AnthropicRequest {
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

interface AnthropicStreamEvent {
	type: string
	[key: string]: any
}

function convertOpenAIToAnthropicMessages(messages: ChatCompletionMessageParam[]): {
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

function convertOpenAIToolsToAnthropic(tools?: OpenAI.Chat.Completions.ChatCompletionTool[]):
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

async function* convertAnthropicStreamToOpenAI(
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

export async function copilot(
	scriptOptions: CopilotOptions,
	generatedCode: Writable<string>,
	abortController: AbortController,
	generatedExplanation?: Writable<string>
) {
	const { prompt, systemPrompt } = await getPrompts(scriptOptions)

	const completion = await getCompletion(
		[
			{
				role: 'system',
				content: systemPrompt
			},
			{
				role: 'user',
				content: prompt
			}
		],
		abortController
	)

	let response = ''
	let code = ''
	for await (const part of completion) {
		response += getResponseFromEvent(part)
		let match = response.match(/```[a-zA-Z]+\n([\s\S]*?)\n```/)

		if (match) {
			// if we have a full code block
			code = match[1]
			generatedCode.set(code)

			if (scriptOptions.type === 'fix') {
				//  in fix mode, check for explanation
				let explanationMatch = response.match(/<explanation>([\s\S]+)<\/explanation>/)

				if (explanationMatch) {
					const explanation = explanationMatch[1].trim()
					generatedExplanation?.set(explanation)
					break
				}

				explanationMatch = response.match(/<explanation>([\s\S]+)/)

				if (!explanationMatch) {
					continue
				}

				const explanation = explanationMatch[1].replace(/<\/?e?x?p?l?a?n?a?t?i?o?n?>?$/, '').trim()

				generatedExplanation?.set(explanation)

				continue
			} else {
				// otherwise stop generating
				break
			}
		}

		// partial code block, keep going
		match = response.match(/```[a-zA-Z]+\n([\s\S]*)/)

		if (!match) {
			continue
		}

		code = match[1]
		if (!code.endsWith('`')) {
			// skip displaying if possible that part of three ticks (end of code block)s
			generatedCode.set(code)
		}
	}

	// make sure we display the latest and complete code
	generatedCode.set(code)

	if (code.length === 0) {
		throw new Error('No code block found')
	}

	return code
}

function getStringEndDelta(prev: string, now: string) {
	return now.slice(prev.length)
}

export async function deltaCodeCompletion(
	messages: ChatCompletionMessageParam[],
	generatedCodeDelta: Writable<string>,
	abortController: AbortController
) {
	const completion = await getCompletion(messages, abortController)

	let response = ''
	let code = ''
	let delta = ''
	for await (const part of completion) {
		response += getResponseFromEvent(part)
		let match = response.match(/```[a-zA-Z]+\n([\s\S]*?)\n```/)

		if (match) {
			// if we have a full code block
			delta = getStringEndDelta(code, match[1])
			code = match[1]
			generatedCodeDelta.set(delta)

			break
		}

		// partial code block, keep going
		match = response.match(/```[a-zA-Z]+\n([\s\S]*)/)

		if (!match) {
			continue
		}

		if (!match[1].endsWith('`')) {
			// skip udpating if possible that part of three ticks (end of code block)s
			delta = getStringEndDelta(code, match[1])
			generatedCodeDelta.set(delta)
			code = match[1]
		}
	}

	if (code.length === 0) {
		throw new Error('No code block found')
	}

	return code
}
