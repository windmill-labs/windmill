import { OpenAI } from 'openai'
import { OpenAPI, ResourceService, type Script } from '../../gen'
import type { Writable } from 'svelte/store'
import { Anthropic } from '@anthropic-ai/sdk'
import type { DBSchema, GraphqlSchema, SQLSchema } from '$lib/stores'
import { formatResourceTypes } from './utils'
import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import { Mistral } from '@mistralai/mistralai'
import { buildClientSchema, printSchema } from 'graphql'
import type {
	ChatCompletionCreateParamsStreaming,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'

import type { MessageCreateParams, MessageParam } from '@anthropic-ai/sdk/resources/messages.mjs'
import type { ChatCompletionRequest } from '@mistralai/mistralai/models/components/chatcompletionrequest'
import type {
	SystemMessage,
	UserMessage,
	AssistantMessage,
	ToolMessage,
	CompletionEvent,
	ContentChunk
} from '@mistralai/mistralai/models/components'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

export type AiProviderTypes = 'openai' | 'anthropic' | 'mistral'

interface AiProvider {
	init: (workspace: string, token?: string) => void
}

class WorkspacedMistral implements AiProvider {
	private client: Mistral | undefined

	init(workspace: string, token?: string) {
		if (!this.client) {
			this.client = initWorkspaceAiProvider(workspace, token, 'mistral') as unknown as Mistral
		}
	}

	getClient() {
		if (!this.client) {
			throw new Error('AnthropicAi not initialized')
		}
		return this.client
	}
}

export namespace MistralAi {
	export let workspacedMistral = new WorkspacedMistral()

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

class WorkspacedAnthropic implements AiProvider {
	private client: Anthropic | undefined

	init(workspace: string, token: string | undefined = undefined) {
		if (!this.client) {
			this.client = initWorkspaceAiProvider(workspace, token, 'anthropic') as unknown as Anthropic
		}
	}

	getClient() {
		if (!this.client) {
			throw new Error('AnthropicAi not initialized')
		}
		return this.client
	}
}

export namespace AnthropicAi {
	export let workspacedAnthropic = new WorkspacedAnthropic()

	export const config: MessageCreateParams = {
		temperature: 0,
		max_tokens: 8192,
		model: 'claude-3-5-sonnet-20241022',
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

class WorkspacedOpenai implements AiProvider {
	private client: OpenAI | undefined

	init(workspace: string, token: string | undefined = undefined) {
		if (!this.client) {
			this.client = initWorkspaceAiProvider(workspace, token, 'openai') as unknown as OpenAI
		}
	}

	getClient() {
		if (!this.client) {
			throw new Error('OpenAI not initialized')
		}
		return this.client
	}
}

export namespace OpenAi {
	export let workspacedOpenai = new WorkspacedOpenai()

	export const openaiConfig: ChatCompletionCreateParamsStreaming = {
		temperature: 0,
		max_tokens: 16384,
		model: 'gpt-4o-2024-08-06',
		seed: 42,
		stream: true,
		messages: []
	}

	export function retrieveTextValue(part: OpenAI.Chat.Completions.ChatCompletionChunk) {
		return part.choices[0]?.delta?.content || ''
	}
}

function initWorkspaceAiProvider(
	workspace: string,
	token: string | undefined = undefined,
	aiProvider: AiProviderTypes
): Anthropic | OpenAI | Mistral {
	const baseURL = `${location.origin}${OpenAPI.BASE}/w/${workspace}/ai/proxy`
	let client
	switch (aiProvider) {
		case 'openai': {
			client = new OpenAI({
				baseURL,
				apiKey: 'fake-key',
				defaultHeaders: {
					Authorization: token ? `Bearer ${token}` : ''
				},
				dangerouslyAllowBrowser: true
			})
			break
		}
		case 'anthropic': {
			client = new Anthropic({
				baseURL,
				apiKey: 'fake-key',
				defaultHeaders: {
					Authorization: token ? `Bearer ${token}` : ''
				},
				dangerouslyAllowBrowser: true
			})
			break
		}
		case 'mistral': {
			client = new Mistral({
				serverURL: baseURL
			})
		}
	}
	return client
}

export async function testKey({
	apiKey,
	abortController,
	messages,
	aiProvider
}: {
	apiKey?: string
	messages: ChatCompletionMessageParam[]
	abortController: AbortController
	aiProvider: AiProviderTypes
}) {
	if (apiKey) {
		switch (aiProvider) {
			case 'openai': {
				const openai = new OpenAI({
					apiKey,
					dangerouslyAllowBrowser: true
				})
				await openai.chat.completions.create(
					{
						...OpenAi.openaiConfig,
						messages,
						stream: false
					},
					{
						signal: abortController.signal
					}
				)
				break
			}
			case 'anthropic': {
				const anthropic = new Anthropic({
					apiKey,
					dangerouslyAllowBrowser: true
				})
				const [, anthropicMessages] = AnthropicAi.getSystemPromptAndArrayMessages(messages)
				await anthropic.messages.create(
					{
						...AnthropicAi.config,
						messages: anthropicMessages,
						stream: false
					},
					{
						signal: abortController.signal
					}
				)
				break
			}
			case 'mistral': {
				const mistral = new Mistral({
					apiKey
				})
				await mistral.chat.complete(
					{
						...MistralAi.mistralConfig,
						model: 'codestral-latest',
						stream: false,
						messages: messages as MistralAi.MistralParamsMessage[]
					},
					{
						fetchOptions: {
							signal: abortController.signal,
							headers: {
								'content-type': 'application/json'
							}
						}
					}
				)
				break
			}
		}
	} else {
		await getNonStreamingCompletion(messages, abortController, aiProvider, undefined, true)
	}
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
		['postgresql', 'mysql', 'snowflake', 'bigquery', 'mssql', 'graphql'].includes(language) && // make sure we are using a SQL/query language
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
	aiProvider: AiProviderTypes,
	model = OpenAi.openaiConfig.model,
	noCache?: boolean
) {
	let response: string | undefined = ''
	const queryOptions = {
		query: {
			no_cache: noCache
		},
		signal: abortController.signal
	}
	switch (aiProvider) {
		case 'openai': {
			const openaiClient = OpenAi.workspacedOpenai.getClient()
			const completion = await openaiClient.chat.completions.create(
				{
					...OpenAi.openaiConfig,
					messages,
					stream: false,
					model
				},
				queryOptions
			)
			response = completion.choices[0]?.message.content || ''
			break
		}
		case 'anthropic': {
			const anthropicClient = AnthropicAi.workspacedAnthropic.getClient()
			const [system, anthropicMessages] = AnthropicAi.getSystemPromptAndArrayMessages(messages)
			const message = await anthropicClient.messages.create(
				{
					...AnthropicAi.config,
					system,
					messages: anthropicMessages,
					stream: false
				},
				queryOptions
			)
			response = message.content[0].type === 'text' ? message.content[0].text : ''
			break
		}
		case 'mistral': {
			const mistralClient = MistralAi.workspacedMistral.getClient()
			const message = await mistralClient.chat.complete(
				{
					...MistralAi.mistralConfig,
					model: 'codestral-latest',
					stream: false,
					messages: messages as MistralAi.MistralParamsMessage[]
				},
				{
					fetchOptions: {
						signal: abortController.signal,
						cache: 'no-store'
					}
				}
			)
			response = MistralAi.retrieveTextValue(message.choices && message.choices[0].message.content)
			break
		}
	}
	return response
}

export async function getCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	aiProvider: AiProviderTypes,
	model = OpenAi.openaiConfig.model
) {
	switch (aiProvider) {
		case 'anthropic': {
			const anthropicClient = AnthropicAi.workspacedAnthropic.getClient()
			const [system, anthropicMessages] = AnthropicAi.getSystemPromptAndArrayMessages(messages)

			const completion = await anthropicClient.messages.create(
				{
					...AnthropicAi.config,
					system,
					messages: anthropicMessages,
					stream: true
				},
				{ signal: abortController.signal }
			)
			return completion
		}
		case 'openai': {
			const openaiClient = OpenAi.workspacedOpenai.getClient()
			const completion = await openaiClient.chat.completions.create(
				{
					...OpenAi.openaiConfig,
					messages,
					model
				},
				{
					signal: abortController.signal
				}
			)
			return completion
		}
		case 'mistral': {
			const mistralClient = MistralAi.workspacedMistral.getClient()
			const message = await mistralClient.chat.stream(
				{
					...MistralAi.mistralConfig,
					model: 'codestral-latest',
					messages: messages as MistralAi.MistralParamsMessage[]
				},
				{
					fetchOptions: {
						signal: abortController.signal,
						cache: 'no-store'
					}
				}
			)
			return message
		}
	}
}

function isCompletionEvent(
	message:
		| Anthropic.Messages.RawMessageStreamEvent
		| OpenAI.Chat.Completions.ChatCompletionChunk
		| CompletionEvent
): message is CompletionEvent {
	return 'data' in message
}
function isChatCompletionChunk(
	response:
		| Anthropic.Messages.RawMessageStreamEvent
		| OpenAI.Chat.Completions.ChatCompletionChunk
		| CompletionEvent
): response is OpenAI.Chat.Completions.ChatCompletionChunk {
	return 'choices' in response
}

export function getResponseFromEvent(
	part:
		| Anthropic.Messages.RawMessageStreamEvent
		| OpenAI.Chat.Completions.ChatCompletionChunk
		| CompletionEvent
): string {
	if (isChatCompletionChunk(part)) {
		return OpenAi.retrieveTextValue(part)
	} else if (isCompletionEvent(part)) {
		return MistralAi.retrieveTextValue(part.data.choices[0].delta.content)
	}
	return AnthropicAi.retrieveTextValue(part)
}

export async function copilot(
	scriptOptions: CopilotOptions,
	generatedCode: Writable<string>,
	abortController: AbortController,
	aiProvider: AiProviderTypes,
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
	abortController: AbortController,
	aiProvider: AiProviderTypes
) {
	const completion = await getCompletion(messages, abortController, aiProvider)

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
