import type { AIProvider, AIProviderModel } from '$lib/gen'
import { workspaceStore, type DBSchema, type GraphqlSchema, type SQLSchema } from '$lib/stores'
import { buildClientSchema, printSchema } from 'graphql'
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
import { formatResourceTypes } from './utils'
import { z } from 'zod'
import { processToolCall, type Tool, type ToolCallbacks } from './chat/shared'
import {
	getNonStreamingOpenAIResponsesCompletion,
	getOpenAIResponsesCompletionStream
} from './chat/openai-responses'
import { convertOpenAIToAnthropicMessages } from './chat/anthropic'
import type { Stream } from 'openai/core/streaming.mjs'
import { generateRandomString } from '$lib/utils'
import { copilotInfo, getCurrentModel } from '$lib/aiStore'

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
	azure_openai: {
		label: 'Azure OpenAI',
		defaultModels: OPENAI_MODELS
	},
	anthropic: {
		label: 'Anthropic',
		defaultModels: ['claude-sonnet-4-6', 'claude-sonnet-4-6/thinking', 'claude-3-5-haiku-latest']
	},
	mistral: {
		label: 'Mistral',
		defaultModels: ['codestral-latest']
	},
	deepseek: {
		label: 'DeepSeek',
		defaultModels: ['deepseek-chat', 'deepseek-reasoner']
	},
	googleai: {
		label: 'Google AI',
		defaultModels: ['gemini-2.0-flash', 'gemini-1.5-flash', 'gemini-1.5-pro']
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
	} else if ((provider === 'azure_openai' || provider === 'openai') && model.startsWith('o')) {
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

export function getModelContextWindow(model: string) {
	if (model.includes('gpt-4.1') || model.includes('gemini')) {
		return 1000000
	} else if (model.includes('gpt-5')) {
		return 400000
	} else if (model.includes('gpt-4o') || model.includes('llama-3.3')) {
		return 128000
	} else if (model.includes('claude') || model.includes('o4-mini') || model.includes('o3')) {
		return 200000
	} else if (model.includes('codestral')) {
		return 32000
	} else {
		return 128000
	}
}

function getModelSpecificConfig(
	modelProvider: AIProviderModel,
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[]
) {
	const defaultMaxTokens = getModelMaxTokens(modelProvider.provider, modelProvider.model)
	const modelKey = `${modelProvider.provider}:${modelProvider.model}`
	const customMaxTokensStore = get(copilotInfo)?.maxTokensPerModel
	const maxTokens = customMaxTokensStore?.[modelKey] ?? defaultMaxTokens
	if (
		(modelProvider.provider === 'openai' || modelProvider.provider === 'azure_openai') &&
		(modelProvider.model.startsWith('o') || modelProvider.model.startsWith('gpt-5'))
	) {
		return {
			model: modelProvider.model,
			...(tools && tools.length > 0 ? { tools } : {}),
			max_completion_tokens: maxTokens
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

class WorkspacedAIClients {
	private openaiClient: OpenAI | undefined
	private anthropicClient: Anthropic | undefined

	init(workspace: string) {
		this.initOpenai(workspace)
		this.initAnthropic(workspace)
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

	private initAnthropic(workspace: string) {
		const baseURL = this.getBaseURL(workspace)
		this.anthropicClient = new Anthropic({
			baseURL,
			apiKey: 'fake-key',
			dangerouslyAllowBrowser: true
		})
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
	const modelToTest = model ?? AI_PROVIDERS[aiProvider].defaultModels[0]

	if (!modelToTest) {
		throw new Error('Missing a model to test')
	}

	// Use Anthropic SDK for Anthropic provider
	if (aiProvider === 'anthropic') {
		await testAnthropicKey({
			apiKey,
			resourcePath,
			model: modelToTest,
			abortController,
			messages
		})
		return
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

async function testAnthropicKey({
	apiKey,
	resourcePath,
	model,
	abortController,
	messages
}: {
	apiKey?: string
	resourcePath?: string
	model: string
	abortController: AbortController
	messages: ChatCompletionMessageParam[]
}) {
	const { system, messages: anthropicMessages } = convertOpenAIToAnthropicMessages(messages)

	const headers: Record<string, string> = {
		'X-Provider': 'anthropic',
		'anthropic-version': '2023-06-01',
		'X-Anthropic-SDK': 'true'
	}

	if (resourcePath) {
		headers['X-Resource-Path'] = resourcePath
	} else if (apiKey) {
		headers['X-API-Key'] = apiKey
	}

	const anthropicClient = apiKey
		? new Anthropic({
				baseURL: `${location.origin}${OpenAPI.BASE}/ai/proxy`,
				apiKey: 'fake-key',
				dangerouslyAllowBrowser: true
			})
		: workspaceAIClients.getAnthropicClient()

	await anthropicClient.messages.create(
		{
			model,
			max_tokens: 100,
			messages: anthropicMessages,
			...(system && { system })
		},
		{
			signal: abortController.signal,
			headers
		}
	)
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

export function getProviderAndCompletionConfig<K extends boolean>({
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

	// Use Responses API for OpenAI and Azure OpenAI
	if (provider === 'openai' || provider === 'azure_openai') {
		try {
			const response = await getNonStreamingOpenAIResponsesCompletion(
				messages,
				abortController,
				testOptions
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
	tools?: OpenAI.Chat.Completions.ChatCompletionTool[],
	options?: {
		forceCompletions?: boolean
	}
): Promise<Stream<ChatCompletionChunk>> {
	const { provider, config } = getProviderAndCompletionConfig({ messages, stream: true, tools })

	// Use Responses API for OpenAI and Azure OpenAI
	if ((provider === 'openai' || provider === 'azure_openai') && !options?.forceCompletions) {
		try {
			const stream = getOpenAIResponsesCompletionStream(messages, abortController, tools) as any
			return stream
		} catch (error) {
			console.error('Error using Responses API:', error)
		}
	}

	// Use Completions API for other providers
	const openaiClient = workspaceAIClients.getOpenaiClient()
	const completion = openaiClient.chat.completions.create(config, {
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
	_abortController?: AbortController // unused, for signature compatibility with parseAnthropicCompletion
): Promise<boolean> {
	const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}
	let malformedFunctionCallError = false

	let answer = ''
	for await (const chunk of completion) {
		if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
			continue
		}
		const c = chunk as ChatCompletionChunk

		// Check for malformed function call error (e.g. from Gemini models)
		const finishReason = c.choices[0].finish_reason
		if (
			finishReason &&
			typeof finishReason === 'string' &&
			finishReason.includes('MALFORMED_FUNCTION_CALL')
		) {
			malformedFunctionCallError = true
		}

		const delta = c.choices[0].delta.content
		if (delta) {
			answer += delta
			callbacks.onNewToken(delta)
		}
		const toolCalls = c.choices[0].delta.tool_calls || []
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
						parameters: parameters
					})
				}
			}
		}
	}

	if (answer) {
		const toAdd = { role: 'assistant' as const, content: answer }
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

	const toolCalls = Object.values(finalToolCalls).filter(
		(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
	) as ChatCompletionMessageFunctionToolCall[]

	if (toolCalls.length > 0) {
		const toAdd = {
			role: 'assistant' as const,
			tool_calls: toolCalls.map((t) => ({
				...t,
				function: {
					...t.function,
					arguments: t.function.arguments || '{}'
				}
			}))
		}
		messages.push(toAdd)
		addedMessages.push(toAdd)
		for (const toolCall of toolCalls) {
			const messageToAdd = await processToolCall({
				tools,
				toolCall,
				helpers,
				toolCallbacks: callbacks
			})
			messages.push(messageToAdd)
			addedMessages.push(messageToAdd)
		}
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
		return false
	}
	return true
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
