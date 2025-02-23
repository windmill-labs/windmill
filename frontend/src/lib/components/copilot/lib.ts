import type { AIProvider } from '$lib/gen'
import {
	copilotInfo,
	copilotSessionModel,
	type DBSchema,
	type GraphqlSchema,
	type SQLSchema
} from '$lib/stores'
import { Anthropic } from '@anthropic-ai/sdk'
import { Mistral } from '@mistralai/mistralai'
import { buildClientSchema, printSchema } from 'graphql'
import { OpenAI } from 'openai'
import type {
	ChatCompletionCreateParamsStreaming,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'
import { get, type Writable } from 'svelte/store'
import { OpenAPI, ResourceService, type Script } from '../../gen'
import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import { formatResourceTypes } from './utils'

import type { MessageCreateParams, MessageParam } from '@anthropic-ai/sdk/resources/messages.mjs'
import type {
	AssistantMessage,
	CompletionEvent,
	ContentChunk,
	SystemMessage,
	ToolMessage,
	UserMessage
} from '@mistralai/mistralai/models/components'
import type { ChatCompletionRequest } from '@mistralai/mistralai/models/components/chatcompletionrequest'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

// need at least one model for each provider except customai
export const AI_DEFAULT_MODELS: Record<AIProvider, string[]> = {
	openai: ['gpt-4o', 'gpt-4o-mini'],
	anthropic: ['claude-3-5-sonnet-latest', 'claude-3-5-haiku-latest'],
	mistral: ['codestral-latest'],
	deepseek: ['deepseek-chat', 'deepseek-reasoner'],
	googleai: ['gemini-1.5-pro', 'gemini-2.0-flash', 'gemini-1.5-flash'],
	groq: ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant'],
	openrouter: ['meta-llama/llama-3.2-3b-instruct:free'],
	customai: []
}

export const OPENAI_COMPATIBLE_BASE_URLS = {
	groq: 'https://api.groq.com/openai/v1',
	openrouter: 'https://openrouter.ai/api/v1',
	deepseek: 'https://api.deepseek.com/v1',
	googleai: 'https://generativelanguage.googleapis.com/v1beta/openai'
} as const

function prepareOpenaiCompatibleMessages(
	aiProvider: AIProvider,
	messages: ChatCompletionMessageParam[]
) {
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

const DEFAULT_COMPLETION_CONFIG: ChatCompletionCreateParamsStreaming = {
	model: '',
	max_tokens: 8192, //TODO: make this dynamic
	temperature: 0,
	seed: 42,
	stream: true,
	messages: []
}

export const OPENAI_COMPATIBLE_COMPLETION_CONFIG = {
	groq: DEFAULT_COMPLETION_CONFIG,
	openrouter: DEFAULT_COMPLETION_CONFIG,
	deepseek: DEFAULT_COMPLETION_CONFIG,
	googleai: {
		...DEFAULT_COMPLETION_CONFIG,
		seed: undefined // not supported by gemini
	} as ChatCompletionCreateParamsStreaming
} as const

class WorkspacedAIClients {
	private openaiClient: OpenAI | undefined
	private anthropicClient: Anthropic | undefined
	private mistralClient: Mistral | undefined

	init(workspace: string) {
		this.initOpenai(workspace)
		this.initAnthropic(workspace)
		this.initMistral(workspace)
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

	private initMistral(workspace: string) {
		const baseURL = this.getBaseURL(workspace)
		this.mistralClient = new Mistral({
			serverURL: baseURL
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

	getMistralClient() {
		if (!this.mistralClient) {
			throw new Error('Mistral not initialized')
		}
		return this.mistralClient
	}
}

export const workspaceAIClients = new WorkspacedAIClients()

namespace MistralAI {
	export const mistralConfig: ChatCompletionRequest = {
		temperature: 0,
		model: null,
		maxTokens: 32000,
		messages: []
	}

	export type MistralParamsMessage =
		| (SystemMessage & { role: 'system' })
		| (UserMessage & { role: 'user' })
		| (AssistantMessage & { role: 'assistant' })
		| (ToolMessage & { role: 'tool' })

	export function retrieveTextValue(chunks: string | ContentChunk[] | null | undefined): string {
		let response = ''
		if (Array.isArray(chunks)) {
			for (const chunk of chunks) {
				if (chunk.type === 'text') {
					response += chunk.text
				}
			}
			return response
		}
		return chunks as string
	}
}

export namespace AnthropicAI {
	export const config: MessageCreateParams = {
		temperature: 0,
		max_tokens: 8192,
		model: '',
		messages: []
	}

	export function getSystemPromptAndArrayMessages(
		messages: ChatCompletionMessageParam[]
	): [string, MessageParam[]] {
		let system: string | undefined = undefined
		if (messages[0].role == 'system') {
			system = messages[0].content as string
			messages.shift()
		}
		const anthropicMessages: MessageParam[] = messages.map((message) => {
			return {
				role: message.role == 'user' ? 'user' : 'assistant',
				content: message.content as string
			}
		})
		return [system as string, anthropicMessages ?? []]
	}

	export function retrieveTextValue(part: Anthropic.Messages.RawMessageStreamEvent) {
		let response = ''
		if (part.type == 'content_block_delta') {
			if (part.delta.type == 'text_delta') {
				response = part.delta.text
			} else {
				response = part.delta.partial_json
			}
		}
		return response
	}
}

namespace OpenAi {
	export const openaiConfig: ChatCompletionCreateParamsStreaming = {
		temperature: 0,
		max_tokens: 16384,
		model: '',
		seed: 42,
		stream: true,
		messages: []
	}

	export function retrieveTextValue(part: OpenAI.Chat.Completions.ChatCompletionChunk) {
		return part.choices[0]?.delta?.content || ''
	}
}

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

	await getNonStreamingCompletion(
		messages,
		abortController,
		aiProvider,
		apiKey,
		resourcePath,
		modelToTest
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

export async function getNonStreamingCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	aiProvider: AIProvider,
	apiKey?: string, // testing API KEY directly from the frontend
	resourcePath?: string, // testing resource path passed as a header to the backend proxy
	forceModel?: string
) {
	let response: string | undefined = ''
	let model = forceModel

	if (!model) {
		model = get(copilotSessionModel)
		let info = get(copilotInfo)
		const { ai_models: aiModels } = info

		if (!model || !aiModels.includes(model)) {
			console.warn('Invalid model, using default model:', aiModels[0])
			model = aiModels[0]
		}
	}

	if (!model) {
		throw new Error('No model found')
	}

	const fetchOptions: {
		signal: AbortSignal
		headers?: Record<string, string>
	} = {
		signal: abortController.signal
	}
	if (resourcePath) {
		fetchOptions.headers = {
			'X-Resource-Path': resourcePath
		}
	}
	switch (aiProvider) {
		case 'anthropic': {
			const anthropicClient = apiKey
				? new Anthropic({
						apiKey,
						dangerouslyAllowBrowser: true
				  })
				: workspaceAIClients.getAnthropicClient()
			const [system, anthropicMessages] = AnthropicAI.getSystemPromptAndArrayMessages(messages)
			const message = await anthropicClient.messages.create(
				{
					...AnthropicAI.config,
					system,
					model,
					messages: anthropicMessages,
					stream: false
				},
				fetchOptions
			)
			response = message.content[0].type === 'text' ? message.content[0].text : ''
			break
		}
		case 'mistral': {
			const mistralClient = apiKey
				? new Mistral({
						apiKey
				  })
				: workspaceAIClients.getMistralClient()
			const message = await mistralClient.chat.complete(
				{
					...MistralAI.mistralConfig,
					model,
					stream: false,
					messages: messages as MistralAI.MistralParamsMessage[]
				},
				{
					fetchOptions: fetchOptions
				}
			)
			response = MistralAI.retrieveTextValue(message.choices && message.choices[0].message.content)
			break
		}
		default: {
			if (aiProvider === 'customai' && apiKey) {
				throw new Error('Cannot test API key for Custom AI, only resource path is supported')
			}
			const baseURL = OPENAI_COMPATIBLE_BASE_URLS[aiProvider]

			if (apiKey && aiProvider !== 'openai' && !baseURL) {
				throw new Error('No base URL for this provider: ' + aiProvider)
			}
			const openaiClient = apiKey
				? new OpenAI({
						apiKey,
						baseURL,
						dangerouslyAllowBrowser: true
				  })
				: workspaceAIClients.getOpenaiClient()
			const config =
				aiProvider === 'openai'
					? OpenAi.openaiConfig
					: OPENAI_COMPATIBLE_COMPLETION_CONFIG[aiProvider]
			if (!config) {
				throw new Error('No config for this provider: ' + aiProvider)
			}
			const processedMessages = prepareOpenaiCompatibleMessages(aiProvider, messages)
			const completion = await openaiClient.chat.completions.create(
				{
					...config,
					messages: processedMessages,
					model,
					stream: false
				},
				fetchOptions
			)
			response = completion.choices[0]?.message.content || ''
		}
	}
	return response
}

export async function getCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	aiProvider: AIProvider
) {
	let model = get(copilotSessionModel)
	let info = get(copilotInfo)
	const { ai_models: aiModels } = info

	if (!model || !aiModels.includes(model)) {
		console.warn('Invalid model, using default model:', aiModels[0])
		model = aiModels[0]
	}

	if (!model) {
		throw new Error('No model found')
	}

	switch (aiProvider) {
		case 'anthropic': {
			const anthropicClient = workspaceAIClients.getAnthropicClient()
			const [system, anthropicMessages] = AnthropicAI.getSystemPromptAndArrayMessages(messages)

			const completion = await anthropicClient.messages.create(
				{
					...AnthropicAI.config,
					model,
					system,
					messages: anthropicMessages,
					stream: true
				},
				{ signal: abortController.signal }
			)
			return completion
		}
		case 'mistral': {
			const mistralClient = workspaceAIClients.getMistralClient()
			const message = await mistralClient.chat.stream(
				{
					...MistralAI.mistralConfig,
					model,
					messages: messages as MistralAI.MistralParamsMessage[]
				},
				{
					fetchOptions: {
						signal: abortController.signal
					}
				}
			)
			return message
		}
		default: {
			const openaiClient = workspaceAIClients.getOpenaiClient()
			const config: ChatCompletionCreateParamsStreaming =
				aiProvider === 'openai'
					? OpenAi.openaiConfig
					: OPENAI_COMPATIBLE_COMPLETION_CONFIG[aiProvider]
			if (!config) {
				throw new Error('No config for this provider: ' + aiProvider)
			}
			const processedMessages = prepareOpenaiCompatibleMessages(aiProvider, messages)
			const completion = await openaiClient.chat.completions.create(
				{
					...config,
					model,
					messages: processedMessages
				},
				{
					signal: abortController.signal
				}
			)
			return completion
		}
	}
}

export function getResponseFromEvent(
	part:
		| Anthropic.Messages.RawMessageStreamEvent
		| OpenAI.Chat.Completions.ChatCompletionChunk
		| CompletionEvent,
	aiProvider: AIProvider
): string {
	switch (aiProvider) {
		case 'anthropic': {
			const messages = part as Anthropic.Messages.RawMessageStreamEvent
			return AnthropicAI.retrieveTextValue(messages)
		}
		case 'mistral': {
			const messages = part as CompletionEvent
			return MistralAI.retrieveTextValue(messages.data.choices[0].delta.content)
		}
		default:
			const messages = part as OpenAI.Chat.Completions.ChatCompletionChunk
			return OpenAi.retrieveTextValue(messages)
	}
}

export async function copilot(
	scriptOptions: CopilotOptions,
	generatedCode: Writable<string>,
	abortController: AbortController,
	aiProvider: AIProvider,
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
		abortController,
		aiProvider
	)

	let response = ''
	let code = ''
	for await (const part of completion) {
		response += getResponseFromEvent(part, aiProvider)
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
	abortController: AbortController,
	aiProvider: AIProvider
) {
	const completion = await getCompletion(messages, abortController, aiProvider)

	let response = ''
	let code = ''
	let delta = ''
	for await (const part of completion) {
		response += getResponseFromEvent(part, aiProvider)
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
