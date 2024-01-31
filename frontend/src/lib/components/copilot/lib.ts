import { OpenAI } from 'openai'
import { OpenAPI, ResourceService, Script } from '../../gen'
import type { Writable } from 'svelte/store'

import type { DBSchema } from '$lib/stores'
import { formatResourceTypes } from './utils'

import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import type {
	ChatCompletionCreateParamsStreaming,
	ChatCompletionMessageParam
} from 'openai/resources/chat'
import { buildClientSchema, printSchema } from 'graphql'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

const openaiConfig: ChatCompletionCreateParamsStreaming = {
	temperature: 0,
	max_tokens: 4096,
	model: 'gpt-4-0125-preview',
	seed: 42,
	stream: true,
	messages: []
}

class WorkspacedOpenai {
	private client: OpenAI | undefined

	init(workspace: string) {
		const baseURL = `${location.origin}${OpenAPI.BASE}/w/${workspace}/openai/proxy`
		this.client = new OpenAI({
			baseURL,
			apiKey: 'fakekey',
			defaultHeaders: {
				Authorization: ''
			},
			dangerouslyAllowBrowser: true
		})
	}

	getClient() {
		if (!this.client) {
			throw new Error('OpenAI not initialized')
		}
		return this.client
	}
}

export let workspacedOpenai = new WorkspacedOpenai()

export async function testKey({
	apiKey,
	abortController,
	messages
}: {
	apiKey?: string
	messages: ChatCompletionMessageParam[]
	abortController: AbortController
}) {
	if (apiKey) {
		const openai = new OpenAI({
			apiKey,
			dangerouslyAllowBrowser: true
		})
		await openai.chat.completions.create(
			{
				...openaiConfig,
				messages,
				stream: false
			},
			{
				signal: abortController.signal
			}
		)
	} else {
		await getNonStreamingCompletion(messages, abortController, undefined, true)
	}
}

interface BaseOptions {
	language: Script.language | 'frontend'
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
	if (['deno', 'bun', 'nativets', 'python3'].includes(scriptOptions.language)) {
		const resourceTypes = await getResourceTypes(scriptOptions)
		const resourceTypesText = formatResourceTypes(
			resourceTypes,
			scriptOptions.language === 'python3' ? 'python3' : 'typescript'
		)
		prompt = prompt.replace('{resourceTypes}', resourceTypesText)
	}
	return prompt
}

function addDBSChema(scriptOptions: CopilotOptions, prompt: string) {
	const { dbSchema, language } = scriptOptions
	if (
		dbSchema &&
		['postgresql', 'mysql', 'snowflake', 'bigquery', 'mssql', 'graphql'].includes(language) && // make sure we are using a SQL/query language
		language === dbSchema.lang // make sure we are using the same language as the schema
	) {
		const { schema, lang } = dbSchema
		if (lang === 'graphql') {
			const graphqlSchema = printSchema(buildClientSchema(schema))
			prompt =
				prompt +
				'\nHere is the GraphQL schema: <schema>\n' +
				JSON.stringify(graphqlSchema) +
				'\n</schema>'
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
			prompt =
				prompt +
				"\nHere's the database schema, each column is in the format [name, type, required, default?]: <dbschema>\n" +
				JSON.stringify(finalSchema) +
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
	model = openaiConfig.model,
	noCache?: boolean
) {
	const openaiClient = workspacedOpenai.getClient()
	const completion = await openaiClient.chat.completions.create(
		{
			...openaiConfig,
			messages,
			stream: false,
			model
		},
		{
			query: {
				no_cache: noCache
			},
			signal: abortController.signal
		}
	)

	// if (completion.usage) {
	// 	const { prompt_tokens, completion_tokens } = completion.usage
	// 	console.log('Cost: ', (prompt_tokens * 0.0015 + completion_tokens * 0.002) / 1000)
	// }

	return completion.choices[0]?.message.content || ''
}

export async function getCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	model = openaiConfig.model
) {
	const openaiClient = workspacedOpenai.getClient()
	const completion = await openaiClient.chat.completions.create(
		{
			...openaiConfig,
			messages,
			model
		},
		{
			signal: abortController.signal
		}
	)

	return completion
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
		response += part.choices[0]?.delta?.content || ''
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
		response += part.choices[0]?.delta?.content || ''
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
