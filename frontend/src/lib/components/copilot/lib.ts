import { OpenAI } from 'openai'
import { OpenAPI } from '../../gen/core/OpenAPI'
import { ResourceService, Script, WorkspaceService } from '../../gen'
import type { Writable } from 'svelte/store'

import { existsOpenaiResourcePath, workspaceStore, type DBSchema } from '$lib/stores'
import { formatResourceTypes } from './utils'

import { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG } from './prompts'
import type {
	CompletionCreateParamsStreaming,
	CreateChatCompletionRequestMessage
} from 'openai/resources/chat'

export const SUPPORTED_LANGUAGES = new Set(Object.keys(GEN_CONFIG.prompts))

const openaiConfig: CompletionCreateParamsStreaming = {
	temperature: 0.5,
	max_tokens: 2048,
	model: 'gpt-4',
	stream: true,
	messages: []
}

let workspace: string | undefined = undefined
let openai: OpenAI | undefined = undefined

workspaceStore.subscribe(async (value) => {
	workspace = value
	const baseURL = `${location.origin}${OpenAPI.BASE}/w/${workspace}/openai/proxy`
	openai = new OpenAI({
		baseURL,
		apiKey: 'fakekey',
		defaultHeaders: {
			Authorization: ''
		},
		dangerouslyAllowBrowser: true
	})
	if (value) {
		try {
			existsOpenaiResourcePath.set(
				await WorkspaceService.existsOpenaiResourcePath({ workspace: value })
			)
		} catch (err) {
			existsOpenaiResourcePath.set(false)
			console.error('Could not get if OpenAI resource exists')
		}
	}
})

interface BaseOptions {
	language: Script.language | 'frontend'
	dbSchema: DBSchema | undefined
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

export async function addResourceTypes(scriptOptions: CopilotOptions, prompt: string) {
	if (!workspace) {
		throw new Error('Workspace not initialized')
	}

	if (['deno', 'bun', 'nativets'].includes(scriptOptions.language)) {
		const resourceTypes = await ResourceService.listResourceType({ workspace })
		const resourceTypesText = formatResourceTypes(resourceTypes, 'typescript')
		prompt = prompt.replace('{resourceTypes}', resourceTypesText)
	} else if (scriptOptions.language === 'python3') {
		const resourceTypes = await ResourceService.listResourceType({ workspace })
		const resourceTypesText = formatResourceTypes(resourceTypes, 'python3')
		prompt = prompt.replace('{resourceTypes}', resourceTypesText)
	}
	return prompt
}

function addDBSChema(scriptOptions: CopilotOptions, prompt: string) {
	const { dbSchema, language } = scriptOptions
	if (
		dbSchema &&
		['postgresql', 'mysql'].includes(language) && // make sure we are using a SQL language
		(dbSchema.lang === 'postgresql' || dbSchema.lang === 'mysql') // make sure we have a SQL schema
	) {
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

		let finalSchema:
			| typeof smallerSchema
			| {
					[tableKey: string]: Array<[string, string, boolean, string?]>
			  } = smallerSchema
		if (lang === 'postgresql' && dbSchema.publicOnly) {
			finalSchema = smallerSchema.public || smallerSchema
		} else if (lang === 'mysql' && Object.keys(smallerSchema).length === 1) {
			finalSchema = smallerSchema[Object.keys(smallerSchema)[0]]
		}
		prompt =
			prompt +
			"\nHere's the database schema, each column is in the format [name, type, required, default?]: " +
			JSON.stringify(finalSchema)
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

function getSampleInteraction(scriptOptions: CopilotOptions) {
	const promptsConfig = PROMPTS_CONFIGS[scriptOptions.type]
	const prompts = promptsConfig.prompts[scriptOptions.language]
	let samplePrompt = prompts.prompt
	if (scriptOptions.type !== 'fix') {
		samplePrompt = samplePrompt.replace('{description}', prompts.example_description)
	}

	if (scriptOptions.type !== 'gen') {
		samplePrompt = samplePrompt.replace('{code}', prompts.example_code)
	}

	if (scriptOptions.type === 'fix') {
		samplePrompt = samplePrompt.replace('{error}', prompts.example_error)
	}
	return { samplePrompt, sampleAnswer: prompts.example_answer }
}

const PROMPTS_CONFIGS = {
	fix: FIX_CONFIG,
	edit: EDIT_CONFIG,
	gen: GEN_CONFIG
}

export async function getNonStreamingCompletion(
	messages: CreateChatCompletionRequestMessage[],
	abortController: AbortController
) {
	if (!openai) {
		throw new Error('OpenAI not initialized')
	}

	const completion = await openai.chat.completions.create(
		{
			...openaiConfig,
			messages,
			stream: false
		},
		{
			signal: abortController.signal
		}
	)

	return completion.choices[0]?.message.content || ''
}

export async function getCompletion(
	messages: CreateChatCompletionRequestMessage[],
	abortController: AbortController
) {
	if (!openai) {
		throw new Error('OpenAI not initialized')
	}

	const completion = await openai.chat.completions.create(
		{
			...openaiConfig,
			messages
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

	const { samplePrompt, sampleAnswer } = getSampleInteraction(scriptOptions)

	const completion = await getCompletion(
		[
			{
				role: 'system',
				content: systemPrompt
			},
			{
				role: 'user',
				content: samplePrompt
			},
			{
				role: 'assistant',
				content: sampleAnswer
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
				let explanationMatch = response.match(/explanation: "(.+)"/i)

				if (explanationMatch) {
					const explanation = explanationMatch[1]
					generatedExplanation?.set(explanation)
					break
				}

				explanationMatch = response.match(/explanation: "(.+)/i)

				if (!explanationMatch) {
					continue
				}

				const explanation = explanationMatch[1]

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
	messages: CreateChatCompletionRequestMessage[],
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
