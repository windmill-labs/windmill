import type { AIProvider, AIProviderModel } from '$lib/gen'
import { workspaceStore, type DBSchema, type SQLSchema } from '$lib/stores'
import type { IntrospectionQuery } from 'graphql'
import OpenAI from 'openai'
import type {
	ChatCompletionChunk,
	ChatCompletionCreateParams,
	ChatCompletionCreateParamsNonStreaming,
	ChatCompletionCreateParamsStreaming,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'
import Anthropic from '@anthropic-ai/sdk'
import { get, type Writable } from 'svelte/store'
import { OpenAPI, ResourceService, type Script } from '../../gen'
import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import { requiresMaxCompletionTokens, usesAnthropicMessagesApi } from './modelConfig'
import { applyReasoningToConfig } from './reasoningRegistry'
import { formatResourceTypes } from './utils'
import {
	appendPendingToolImages,
	processToolCall,
	type Tool,
	type ToolCallbacks
} from './chat/shared'
import { hasValidToolCallArguments } from './chat/toolCallArguments'
import {
	getNonStreamingOpenAIResponsesCompletion,
	getOpenAIResponsesCompletionStream
} from './chat/openai-responses'
import { convertOpenAIToAnthropicMessages } from './chat/anthropic'
import type { Stream } from 'openai/core/streaming.mjs'
import { generateRandomString } from '$lib/utils'
import { copilotInfo, getCurrentModel, getMetadataModel } from '$lib/aiStore'
import {
	emptyChatTokenUsage,
	openAICompletionsUsageToChatTokenUsage,
	type ChatTokenUsage
} from './chat/tokenUsage'
import {
	buildAssistantTextMessage,
	buildAssistantToolCallMessage,
	splitContentDelta,
	getReasoningContentDelta
} from './chat/openaiReasoning'
import { parseFimCompletionChoice } from './fim'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

interface AIProviderDetails {
	label: string
	defaultModels: string[]
}

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

export const AI_PROVIDERS: Record<AIProvider, AIProviderDetails> = {
	openai: {
		label: 'OpenAI',
		defaultModels: OPENAI_MODELS
	},
	anthropic: {
		label: 'Anthropic',
		defaultModels: ['claude-sonnet-4-6', 'claude-3-5-haiku-latest']
	},
	googleai: {
		label: 'Google AI',
		defaultModels: [
			'gemini-2.5-flash',
			'gemini-2.5-pro',
			'gemini-2.5-flash-lite',
			'gemini-3-flash',
			'gemini-3.1-pro',
			'gemini-3.1-flash-lite'
		]
	},
	azure_openai: {
		label: 'Azure OpenAI',
		defaultModels: OPENAI_MODELS
	},
	azure_foundry: {
		label: 'Azure AI Foundry',
		defaultModels: [
			'gpt-4o',
			'gpt-4o-mini',
			'DeepSeek-R1',
			'Llama-3.3-70B-Instruct',
			'Phi-4',
			'Mistral-Large-2411'
		]
	},
	mistral: {
		label: 'Mistral',
		defaultModels: ['codestral-latest']
	},
	deepseek: {
		label: 'DeepSeek',
		defaultModels: ['deepseek-v4-pro', 'deepseek-chat', 'deepseek-reasoner']
	},
	groq: {
		label: 'Groq',
		defaultModels: ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant']
	},
	openrouter: {
		label: 'OpenRouter',
		defaultModels: ['meta-llama/llama-3.2-3b-instruct:free']
	},
	togetherai: {
		label: 'Together AI',
		defaultModels: ['meta-llama/Llama-3.3-70B-Instruct-Turbo']
	},
	aws_bedrock: {
		label: 'AWS Bedrock',
		defaultModels: ['global.anthropic.claude-haiku-4-5-20251001-v1:0']
	},
	customai: {
		label: 'Custom AI',
		defaultModels: []
	}
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
	provider: AIProvider,
	signal?: AbortSignal
): Promise<string[]> {
	// Handle AWS Bedrock separately (needs both foundation-models and inference-profiles)
	if (provider === 'aws_bedrock') {
		const headers = {
			'X-Resource-Path': resourcePath,
			'X-Provider': provider
		}

		// Fetch both foundation models and inference profiles
		const [foundationModelsResp, inferenceProfilesResp] = await Promise.all([
			fetch(`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/foundation-models`, {
				signal,
				headers
			}),
			fetch(`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/inference-profiles`, {
				signal,
				headers
			})
		])

		if (!foundationModelsResp.ok) {
			console.error('Failed to fetch foundation models', foundationModelsResp)
			throw new Error('Failed to fetch foundation models for AWS Bedrock')
		}

		const foundationModelsData = (await foundationModelsResp.json()) as {
			modelSummaries: Array<{
				modelId: string
				modelArn: string
				inputModalities: string[]
				outputModalities: string[]
				inferenceTypesSupported: string[]
			}>
		}

		// Inference profiles fetch might fail in some regions/accounts
		let inferenceProfiles: Array<{
			inferenceProfileId: string
			models: Array<{ modelArn: string }>
		}> = []

		if (inferenceProfilesResp.ok) {
			const inferenceProfilesData = (await inferenceProfilesResp.json()) as {
				inferenceProfileSummaries: Array<{
					inferenceProfileId: string
					models: Array<{ modelArn: string }>
				}>
			}
			inferenceProfiles = inferenceProfilesData.inferenceProfileSummaries || []
		} else {
			console.warn('Failed to fetch inference profiles, will use direct model IDs only')
		}

		// Filter to TEXT-capable models
		const textModels = foundationModelsData.modelSummaries.filter(
			(m) => m.inputModalities?.includes('TEXT') && m.outputModalities?.includes('TEXT')
		)

		const onDemandModels = textModels
			.filter(
				(model) =>
					model.inferenceTypesSupported?.includes('ON_DEMAND') &&
					!model.inferenceTypesSupported?.includes('INFERENCE_PROFILE')
			)
			.map((model) => model.modelId)
		const inferenceModels = inferenceProfiles.map((profile) => profile.inferenceProfileId)
		const modelIds = [...onDemandModels, ...inferenceModels]

		// Sort by default models
		const defaultModels = AI_PROVIDERS[provider]?.defaultModels || []
		return modelIds.sort((a, b) => {
			const aInDefault = defaultModels.includes(a)
			const bInDefault = defaultModels.includes(b)
			if (aInDefault && !bInDefault) return -1
			if (!aInDefault && bInDefault) return 1
			return 0
		})
	}

	// Standard provider handling
	const endpoint = 'models'
	const models = await fetch(
		`${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy/${endpoint}`,
		{
			signal,
			headers: {
				'X-Resource-Path': resourcePath,
				'X-Provider': provider,
				...(provider === 'anthropic' ? { 'anthropic-version': '2023-06-01' } : {})
			}
		}
	)
	if (!models.ok) {
		console.error('Failed to fetch models for provider', provider, models)
		throw new Error(`Failed to fetch models for provider ${provider}`)
	}

	const data = (await models.json()) as { data: ModelResponse[] }
	if (data.data.length > 0) {
		const sortFunc = (provider: AIProvider) => (a: string, b: string) => {
			// First prioritize models in defaultModels array
			const defaultModels = AI_PROVIDERS[provider]?.defaultModels || []
			const aInDefault = defaultModels.includes(a)
			const bInDefault = defaultModels.includes(b)

			if (aInDefault && !bInDefault) return -1
			if (!aInDefault && bInDefault) return 1
			return 0
		}
		switch (provider) {
			case 'openai':
				return data.data
					.filter(
						(m) => m.id.startsWith('gpt-') || m.id.startsWith('o') || m.id.startsWith('codex')
					)
					.map((m) => m.id)
					.sort(sortFunc(provider))
			case 'azure_openai':
				return data.data
					.filter(
						(m) =>
							(m.id.startsWith('gpt-') || m.id.startsWith('o') || m.id.startsWith('codex')) &&
							m.lifecycle_status !== 'deprecated'
					)
					.map((m) => m.id)
					.sort(sortFunc(provider))
			case 'googleai':
				return data.data.map((m) => m.id.split('/')[1]).sort(sortFunc(provider))
			default:
				return data.data.map((m) => m.id).sort(sortFunc(provider))
		}
	}

	return data?.data.map((m) => m.id) ?? []
}

export function getModelMaxTokens(provider: AIProvider, model: string) {
	if (model.includes('gpt-5')) {
		return 128000
	} else if (
		(provider === 'azure_openai' || provider === 'openai' || provider === 'azure_foundry') &&
		model.startsWith('o')
	) {
		return 100000
	} else if (
		model.includes('claude-sonnet') ||
		model.includes('gemini-2.5') ||
		model.includes('claude-haiku')
	) {
		return 64000
	} else if (model.includes('gpt-4.1')) {
		return 32768
	} else if (model.includes('claude-opus')) {
		return 32000
	} else if (model.includes('gpt-4o') || model.includes('codestral')) {
		return 16384
	} else if (model.includes('gpt-4-turbo') || model.includes('gpt-3.5')) {
		return 4096
	}
	return 8192
}

// Resolves the completion token cap for a model: the workspace's per-model
// override when set, otherwise the built-in default. Shared by the OpenAI and
// Anthropic request paths so both honor the same limit. `cap` bounds the result
// (used by short metadata completions, see METADATA_MAX_TOKENS) — a hard ceiling
// that wins over both the workspace override and the default.
function resolveMaxTokens(modelProvider: AIProviderModel, cap?: number): number {
	const defaultMaxTokens = getModelMaxTokens(modelProvider.provider, modelProvider.model)
	const modelKey = `${modelProvider.provider}:${modelProvider.model}`
	let customMaxTokensStore: Record<string, number> | undefined
	try {
		customMaxTokensStore = get(copilotInfo)?.maxTokensPerModel
	} catch {
		// copilotInfo store may not be initialized in vitest
	}
	const resolved = customMaxTokensStore?.[modelKey] ?? defaultMaxTokens
	return cap !== undefined ? Math.min(resolved, cap) : resolved
}

// Token cap for metadata completions (session titles, cron/predicate/step-input
// generation). These outputs are short, and the Anthropic SDK refuses a
// non-streaming request whose max_tokens implies a >10-minute worst case
// (60min × max_tokens / 128000): the model defaults (sonnet/haiku 64000, opus
// 32000) all trip it. Capping keeps every provider's metadata call non-streaming.
export const METADATA_MAX_TOKENS = 4096

function getModelSpecificConfig(
	modelProvider: AIProviderModel,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	maxTokensCap?: number
) {
	const maxTokens = resolveMaxTokens(modelProvider, maxTokensCap)
	if (
		(modelProvider.provider === 'openai' ||
			modelProvider.provider === 'azure_openai' ||
			modelProvider.provider === 'azure_foundry') &&
		requiresMaxCompletionTokens(modelProvider.model)
	) {
		return {
			model: modelProvider.model,
			...(tools && tools.length > 0 ? { tools } : {}),
			max_completion_tokens: maxTokens
		}
	} else {
		return {
			model: modelProvider.model,
			...(tools && tools.length > 0 ? { tools } : {}),
			max_tokens: maxTokens
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
	messages: []
}

export const PROVIDER_COMPLETION_CONFIG_MAP: Record<AIProvider, ChatCompletionCreateParams> = {
	openai: DEFAULT_COMPLETION_CONFIG,
	azure_openai: DEFAULT_COMPLETION_CONFIG,
	azure_foundry: DEFAULT_COMPLETION_CONFIG,
	groq: DEFAULT_COMPLETION_CONFIG,
	openrouter: DEFAULT_COMPLETION_CONFIG,
	togetherai: DEFAULT_COMPLETION_CONFIG,
	deepseek: DEFAULT_COMPLETION_CONFIG,
	customai: DEFAULT_COMPLETION_CONFIG,
	googleai: DEFAULT_COMPLETION_CONFIG,
	mistral: DEFAULT_COMPLETION_CONFIG,
	anthropic: DEFAULT_COMPLETION_CONFIG,
	aws_bedrock: DEFAULT_COMPLETION_CONFIG
} as const

export function getAiProxyBaseURL(workspace?: string): string {
	return workspace
		? `${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy`
		: `${location.origin}${OpenAPI.BASE}/ai/proxy`
}

export function createOpenAIProxyClient(baseURL: string): OpenAI {
	return new OpenAI({
		baseURL,
		apiKey: 'fake-key',
		defaultHeaders: {
			Authorization: OpenAPI.TOKEN ? `Bearer ${OpenAPI.TOKEN}` : ''
		},
		dangerouslyAllowBrowser: true
	})
}

export function createAnthropicProxyClient(baseURL: string): Anthropic {
	return new Anthropic({
		baseURL,
		apiKey: 'fake-key',
		dangerouslyAllowBrowser: true,
		defaultHeaders: OpenAPI.TOKEN ? { Authorization: `Bearer ${OpenAPI.TOKEN}` } : undefined
	})
}

class WorkspacedAIClients {
	private openaiClient: OpenAI | undefined
	private anthropicClient: Anthropic | undefined

	init(workspace: string) {
		this.openaiClient = this.createOpenaiClient(workspace)
		this.anthropicClient = this.createAnthropicClient(workspace)
	}

	createOpenaiClient(workspace: string): OpenAI {
		return createOpenAIProxyClient(getAiProxyBaseURL(workspace))
	}

	createAnthropicClient(workspace: string): Anthropic {
		return createAnthropicProxyClient(getAiProxyBaseURL(workspace))
	}

	getOpenaiClient() {
		if (!this.openaiClient) {
			throw new Error('OpenAI not initialized')
		}
		return this.openaiClient
	}

	getAnthropicClient() {
		if (!this.anthropicClient) {
			throw new Error('Anthropic not initialized')
		}
		return this.anthropicClient
	}
}

export const workspaceAIClients = new WorkspacedAIClients()

export async function testKey({
	apiKey,
	workspace,
	resourcePath,
	model,
	abortController,
	messages,
	aiProvider
}: {
	apiKey?: string
	workspace?: string
	resourcePath?: string
	model: string | undefined
	messages: ChatCompletionMessageParam[]
	abortController: AbortController
	aiProvider: AIProvider
}) {
	if (!apiKey && !resourcePath) {
		throw new Error('API key or resource path is required')
	}
	const modelToTest = model ?? AI_PROVIDERS[aiProvider].defaultModels[0]

	if (!modelToTest) {
		throw new Error('Missing a model to test')
	}

	// getNonStreamingCompletion routes Anthropic-Messages-API models (native
	// Anthropic and Claude on Azure Foundry) through the Anthropic SDK and
	// everything else through OpenAI chat completions, so the test exercises the
	// same request shape the feature actually sends.
	await getNonStreamingCompletion(messages, abortController, {
		apiKey,
		workspace,
		resourcePath,
		forceModelProvider: {
			model: modelToTest,
			provider: aiProvider
		}
	})
}

// Providers served through the Anthropic Messages API (native Anthropic, and
// Claude deployments on Azure Foundry) require the Anthropic SDK request shape:
// OpenAI chat-completions requests fail against them because the proxy forwards
// the body verbatim and, for Foundry, rewrites the URL to the /anthropic/v1
// surface that only serves /messages. This centralizes the client/header/message
// setup so every completion entry point routes them the same way the chat does.
interface AnthropicCompletionParams {
	messages: ChatCompletionMessageParam[]
	modelProvider: AIProviderModel
	abortController: AbortController
	apiKey?: string
	workspace?: string
	resourcePath?: string
	maxTokensCap?: number
}

function buildAnthropicProxyRequest({
	messages,
	modelProvider,
	apiKey,
	workspace,
	resourcePath,
	maxTokensCap
}: Omit<AnthropicCompletionParams, 'abortController'>) {
	const { system, messages: anthropicMessages } = convertOpenAIToAnthropicMessages(messages)

	// X-Provider must be the real provider (e.g. azure_foundry) so the backend
	// resolves the right credentials and Anthropic URL; the SDK headers tell it to
	// route through the Anthropic Messages API.
	const headers: Record<string, string> = {
		'X-Provider': modelProvider.provider,
		'anthropic-version': '2023-06-01',
		'X-Anthropic-SDK': 'true'
	}

	if (resourcePath) {
		headers['X-Resource-Path'] = resourcePath
	} else if (apiKey) {
		headers['X-API-Key'] = apiKey
	}

	const client = apiKey
		? createAnthropicProxyClient(getAiProxyBaseURL())
		: workspace
			? workspaceAIClients.createAnthropicClient(workspace)
			: workspaceAIClients.getAnthropicClient()

	const body = {
		model: modelProvider.model,
		max_tokens: resolveMaxTokens(modelProvider, maxTokensCap),
		messages: anthropicMessages,
		...(system && { system })
	}

	return { client, headers, body }
}

async function getAnthropicNonStreamingCompletion({
	abortController,
	...params
}: AnthropicCompletionParams): Promise<string> {
	const { client, headers, body } = buildAnthropicProxyRequest(params)

	const message = await client.messages.create(body, {
		signal: abortController.signal,
		headers
	})

	return message.content.map((block) => (block.type === 'text' ? block.text : '')).join('')
}

// Adapts an Anthropic Messages stream into the OpenAI ChatCompletionChunk shape
// the completion consumers already iterate, so they need no Anthropic-specific
// handling. Only text deltas are surfaced (these paths don't use tool calls).
function getAnthropicStreamingCompletion({
	abortController,
	...params
}: AnthropicCompletionParams): Stream<ChatCompletionChunk> {
	const { client, headers, body } = buildAnthropicProxyRequest(params)

	const stream = client.messages.stream(body, {
		signal: abortController.signal,
		headers
	})

	async function* toOpenAIChunks(): AsyncGenerator<ChatCompletionChunk> {
		for await (const event of stream) {
			if (event.type === 'content_block_delta' && event.delta.type === 'text_delta') {
				yield {
					id: '',
					object: 'chat.completion.chunk',
					created: 0,
					model: params.modelProvider.model,
					choices: [{ index: 0, delta: { content: event.delta.text }, finish_reason: null }]
				}
			}
		}
	}

	return toOpenAIChunks() as unknown as Stream<ChatCompletionChunk>
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

export function stringifySchema(dbSchema: Omit<SQLSchema, 'stringified'>): string {
	const { schema, lang } = dbSchema
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
		finalSchema = smallerSchema.public || smallerSchema.PUBLIC || smallerSchema.dbo || smallerSchema
	} else if (lang === 'mysql' && Object.keys(smallerSchema).length === 1) {
		finalSchema = smallerSchema[Object.keys(smallerSchema)[0]]
	}
	return JSON.stringify(finalSchema)
}

export async function stringifyGraphqlSchema(schema: unknown): Promise<string> {
	const { buildClientSchema, printSchema } = await import('graphql')
	return printSchema(buildClientSchema(schema as IntrospectionQuery))
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

/**
 * Whether a provider can use native web search automatically in the web chat.
 * Azure OpenAI can expose Responses API `web_search` for some deployments, but
 * it is subscription/admin controlled and routes data through Grounding with
 * Bing, so do not silently enable it until there is explicit Azure-specific UI.
 * Providers behind OpenAI-compatible/native-translation proxy paths have no
 * forwardable native web-search tool.
 */
export function providerSupportsWebSearch(provider: AIProvider | undefined): boolean {
	return provider === 'openai' || provider === 'anthropic'
}

/**
 * Best-effort check that a model can accept image input. There is no per-model vision
 * metadata in the codebase, so this is deliberately permissive: it returns true unless
 * the model is a known text-only one that would 400 on an image part. Used to gate the
 * image-attach affordance and the screenshot follow-up; when unsure it allows the image
 * (the user explicitly attached it — better to try than to silently drop it).
 */
export function modelSupportsVision(
	provider: AIProvider | undefined,
	model: string | undefined
): boolean {
	if (!provider) return true
	const m = (model ?? '').toLowerCase()
	const knownTextOnly =
		m.includes('codestral') ||
		m.includes('deepseek-chat') ||
		m.includes('deepseek-reasoner') ||
		m.includes('deepseek-v3') ||
		m.includes('deepseek-r1') ||
		m.includes('o1-mini') ||
		m.includes('o3-mini') ||
		m.includes('embed')
	return !knownTextOnly
}

export function getProviderAndCompletionConfig<K extends boolean>({
	messages,
	stream,
	tools,
	forceModelProvider,
	maxTokensCap
}: {
	messages: ChatCompletionMessageParam[]
	stream: K
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
	forceModelProvider?: AIProviderModel
	maxTokensCap?: number
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
			...getModelSpecificConfig(modelProvider, tools, maxTokensCap),
			messages: processedMessages,
			stream
		} as any
	}
}

export async function getNonStreamingCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	options?: {
		apiKey?: string // testing API KEY using the global ai proxy
		resourcePath?: string // testing resource path passed as a header to the backend proxy
		workspace?: string // use a specific workspace proxy when testing a workspace resource
		forceModelProvider?: AIProviderModel
		maxTokensCap?: number // hard ceiling on output tokens (see METADATA_MAX_TOKENS)
	}
) {
	const modelProvider = options?.forceModelProvider ?? getCurrentModel()

	if (usesAnthropicMessagesApi(modelProvider.provider, modelProvider.model)) {
		return getAnthropicNonStreamingCompletion({
			messages,
			modelProvider,
			abortController,
			apiKey: options?.apiKey,
			workspace: options?.workspace,
			resourcePath: options?.resourcePath,
			maxTokensCap: options?.maxTokensCap
		})
	}

	let response: string | undefined = ''
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: false,
		forceModelProvider: options?.forceModelProvider,
		maxTokensCap: options?.maxTokensCap
	})

	// Use Responses API for OpenAI and Azure OpenAI
	if (provider === 'openai' || provider === 'azure_openai') {
		try {
			const response = await getNonStreamingOpenAIResponsesCompletion(
				messages,
				abortController,
				options
			)
			return response
		} catch (error) {
			console.error('Error using Responses API:', error)
		}
	}

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
		if (provider === 'customai') {
			throw new Error('Cannot test API key for Custom AI, only resource path is supported')
		}

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

	const completion = await openaiClient.chat.completions.create(config, fetchOptions)
	response = completion.choices?.[0]?.message.content || ''
	return response
}

export async function getNonStreamingMetadataCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController
) {
	return getNonStreamingCompletion(messages, abortController, {
		forceModelProvider: getMetadataModel(),
		maxTokensCap: METADATA_MAX_TOKENS
	})
}

export const FIM_MAX_TOKENS = 256
const FIM_MAX_LINES = 8
export async function getFimCompletion(
	prompt: string,
	suffix: string,
	providerModel: AIProviderModel,
	abortController: AbortController
): Promise<string | undefined> {
	// The Anthropic Messages API has no fill-in-the-middle endpoint, and Foundry
	// Claude deployments don't expose the OpenAI-compatible completions surface the
	// FIM proxy targets. Skip autocomplete for these models rather than issuing a
	// request that can't succeed.
	if (usesAnthropicMessagesApi(providerModel.provider, providerModel.model)) {
		return undefined
	}

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
	const choice = parseFimCompletionChoice(body, providerModel.provider)

	if (choice?.content !== undefined) {
		let lines = choice.content.split('\n')

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
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	options?: {
		forceCompletions?: boolean
		forceModelProvider?: AIProviderModel
		openaiClient?: OpenAI
		reasoningEffort?: string
	}
): Promise<Stream<ChatCompletionChunk>> {
	const modelProvider = options?.forceModelProvider ?? getCurrentModel()

	if (usesAnthropicMessagesApi(modelProvider.provider, modelProvider.model)) {
		return getAnthropicStreamingCompletion({ messages, modelProvider, abortController })
	}

	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: true,
		tools,
		forceModelProvider: options?.forceModelProvider
	})

	// Use Responses API for OpenAI and Azure OpenAI
	if ((provider === 'openai' || provider === 'azure_openai') && !options?.forceCompletions) {
		try {
			const stream = getOpenAIResponsesCompletionStream(messages, abortController, tools, {
				forceModelProvider: options?.forceModelProvider,
				openaiClient: options?.openaiClient,
				reasoningEffort: options?.reasoningEffort
			}) as any
			return stream
		} catch (error) {
			console.error('Error using Responses API:', error)
		}
	}

	// Use Completions API for other providers
	const client = options?.openaiClient ?? workspaceAIClients.getOpenaiClient()
	const completionConfig = applyReasoningToConfig(
		(provider === 'openai' ||
			provider === 'azure_openai' ||
			provider === 'azure_foundry' ||
			provider === 'googleai') &&
			config.stream
			? {
					...config,
					stream_options: {
						...(config.stream_options ?? {}),
						include_usage: true
					}
				}
			: config,
		provider === 'deepseek' ? 'deepseek' : provider === 'mistral' ? 'mistral' : 'completions',
		options?.reasoningEffort
	)
	const completion = client.chat.completions.create(completionConfig, {
		signal: abortController.signal,
		headers: {
			'X-Provider': provider
		}
	})
	return completion
}

function extractFirstJSON(str: string) {
	let depth = 0,
		i = 0
	for (; i < str.length; i++) {
		if (str[i] === '{') depth++
		else if (str[i] === '}' && --depth === 0) break
	}
	return str.slice(0, i + 1)
}

export async function parseOpenAICompletion(
	completion: Stream<ChatCompletionChunk>,
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	},
	messages: ChatCompletionMessageParam[],
	addedMessages: ChatCompletionMessageParam[],
	tools: Tool<any>[],
	helpers: any,
	_abortController?: AbortController, // unused, for signature compatibility with parseAnthropicCompletion
	options?: { workspace?: string; provider?: string }
): Promise<{ shouldContinue: boolean; tokenUsage: ChatTokenUsage }> {
	const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}
	let malformedFunctionCallError = false
	let tokenUsage = emptyChatTokenUsage()

	let answer = ''
	let assistantContent = ''
	let reasoningContent = ''
	let hasReasoningContent = false
	for await (const chunk of completion) {
		if ('usage' in chunk && chunk.usage) {
			tokenUsage = openAICompletionsUsageToChatTokenUsage(chunk.usage)
		}
		if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
			continue
		}
		const c = chunk as ChatCompletionChunk
		const choice = c.choices[0]
		const delta = choice.delta

		// Check for malformed function call error (e.g. from Gemini models)
		const finishReason = choice.finish_reason
		if (
			finishReason &&
			typeof finishReason === 'string' &&
			finishReason.includes('MALFORMED_FUNCTION_CALL')
		) {
			malformedFunctionCallError = true
		}

		// Mistral nests reasoning inside structured content parts; split them out
		// so a content delta never leaks "[object Object]" into the answer.
		const structured = splitContentDelta(delta.content)
		const reasoningDelta = getReasoningContentDelta(delta)
		const reasoningText =
			(typeof reasoningDelta === 'string' ? reasoningDelta : '') + structured.reasoning
		if (typeof reasoningDelta === 'string' || structured.reasoning) {
			hasReasoningContent = true
			reasoningContent += reasoningText
			callbacks.onReasoningDelta?.(reasoningText)
		}

		const contentDelta = structured.text
		if (contentDelta) {
			answer += contentDelta
			assistantContent += contentDelta
			callbacks.onNewToken(contentDelta)
		}
		const toolCalls = delta.tool_calls || []
		if (toolCalls.length > 0 && answer) {
			// if tool calls are present but we have some textual content already, we need to display it to the user first
			callbacks.onMessageEnd()
			answer = ''
		}
		for (let i = 0; i < toolCalls.length; i++) {
			const toolCall = toolCalls[i]
			// Gemini models are missing the index field
			if (
				toolCall.index === undefined ||
				(typeof toolCall.index === 'string' && toolCall.index === '')
			) {
				toolCall.index = i
			}
			// Gemini models are missing the id field
			if (toolCall.id === undefined || (typeof toolCall.id === 'string' && toolCall.id === '')) {
				toolCall.id = generateRandomString()
			}
			const { index } = toolCall
			let finalToolCall = finalToolCalls[index]
			if (!finalToolCall) {
				finalToolCalls[index] = toolCall
			} else {
				if (toolCall.function?.arguments) {
					if (!finalToolCall.function) {
						finalToolCall.function = toolCall.function
					} else {
						finalToolCall.function.arguments =
							(finalToolCall.function.arguments ?? '') + toolCall.function.arguments
						// Make sure we only have one JSON object, else for Gemini models it sometimes results in two JSON objects
						finalToolCall.function.arguments = extractFirstJSON(
							finalToolCall.function.arguments || '{}'
						)
					}
				}
			}
			finalToolCall = finalToolCalls[index]
			if (finalToolCall?.function) {
				const {
					function: { name: funcName },
					id: toolCallId
				} = finalToolCall
				if (funcName && toolCallId) {
					const tool = tools.find((t) => t.def.function.name === funcName)
					if (tool && tool.preAction) {
						tool.preAction({ toolCallbacks: callbacks, toolId: toolCallId })
					}

					const shouldStream = tool?.streamArguments ?? false
					const accumulatedArgs = finalToolCall.function.arguments
					let parameters: any = undefined
					if (accumulatedArgs) {
						try {
							parameters = JSON.parse(accumulatedArgs)
						} catch {
							parameters = accumulatedArgs
						}
					}

					// Display tool call with streaming parameters if enabled
					callbacks.setToolStatus(toolCallId, {
						isLoading: true,
						content: `Calling ${funcName}...`,
						toolName: funcName,
						isStreamingArguments: shouldStream,
						showFade: tool?.showFade,
						showDetails: tool?.showDetails,
						autoCollapseDetails: tool?.autoCollapseDetails,
						parameters: parameters
					})
				}
			}
		}
	}

	const toolCalls = Object.values(finalToolCalls).filter(
		(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
	) as ChatCompletionMessageFunctionToolCall[]

	if (answer && toolCalls.length === 0) {
		const toAdd = buildAssistantTextMessage(answer)
		addedMessages.push(toAdd)
		messages.push(toAdd)
	}

	callbacks.onMessageEnd()

	// Clear streaming state for all tool calls
	for (const toolCall of Object.values(finalToolCalls)) {
		if (toolCall.id) {
			callbacks.setToolStatus(toolCall.id, { isStreamingArguments: false })
		}
	}

	if (toolCalls.length > 0) {
		const invalidToolCallIds = new Set(
			toolCalls.filter((t) => !hasValidToolCallArguments(t.function.arguments)).map((t) => t.id)
		)
		const normalizedToolCalls = toolCalls.map((t) => ({
			...t,
			function: {
				...t.function,
				arguments: invalidToolCallIds.has(t.id) ? '{}' : t.function.arguments || '{}'
			}
		}))
		const toAdd = buildAssistantToolCallMessage({
			content: assistantContent,
			reasoning: {
				hasReasoningContent,
				reasoningContent
			},
			toolCalls: normalizedToolCalls,
			provider: options?.provider
		})
		messages.push(toAdd)
		addedMessages.push(toAdd)
		for (const toolCall of toolCalls) {
			if (invalidToolCallIds.has(toolCall.id)) {
				callbacks.setToolStatus(toolCall.id, {
					isLoading: false,
					isStreamingArguments: false,
					error: 'Tool call arguments were invalid or truncated'
				})
				const messageToAdd = {
					role: 'tool' as const,
					tool_call_id: toolCall.id,
					content:
						'The tool call arguments were invalid or truncated JSON, so the tool was NOT executed. Retry the call; if the arguments were long, split the work into several smaller calls.'
				}
				messages.push(messageToAdd)
				addedMessages.push(messageToAdd)
				continue
			}
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
	} else if (malformedFunctionCallError) {
		// Malformed function call with no tool calls - create artificial tool call to inform AI
		const fakeToolCallId = generateRandomString()

		// Show error status to user
		callbacks.setToolStatus(fakeToolCallId, {
			isLoading: false,
			content: 'Malformed function call',
			error: 'Invalid input given to function call',
			toolName: 'unknown'
		})

		// Add assistant message with fake tool call
		const assistantMessage = {
			role: 'assistant' as const,
			tool_calls: [
				{
					id: fakeToolCallId,
					type: 'function' as const,
					function: {
						name: 'unknown',
						arguments: '{}'
					}
				}
			]
		}
		messages.push(assistantMessage)
		addedMessages.push(assistantMessage)

		// Add tool response telling AI to retry
		const toolResponse = {
			role: 'tool' as const,
			tool_call_id: fakeToolCallId,
			content: 'Invalid input given to function call, MUST TRY WITH SIMPLER ARGUMENTS'
		}
		messages.push(toolResponse)
		addedMessages.push(toolResponse)
	} else {
		return { shouldContinue: false, tokenUsage }
	}
	return { shouldContinue: true, tokenUsage }
}

export function getResponseFromEvent(part: OpenAI.Chat.Completions.ChatCompletionChunk): string {
	return part.choices?.[0]?.delta?.content || ''
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
