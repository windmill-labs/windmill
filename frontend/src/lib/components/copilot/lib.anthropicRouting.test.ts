import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AIProviderModel } from '$lib/gen'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

// getCurrentModel/getMetadataModel are read per call, so a hoisted holder lets
// each test point the routing at a different provider/model.
const h = vi.hoisted(() => ({ currentModel: undefined as AIProviderModel | undefined }))

vi.mock('monaco-editor', () => ({ editor: {} }))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined }
}))

vi.mock('$lib/components/flows/flowTree', () => ({
	findModuleInModules: () => undefined
}))

vi.mock('$lib/gen', () => ({
	OpenAPI: { BASE: '/api', TOKEN: undefined },
	ResourceService: {},
	ScriptService: {},
	FlowService: {},
	JobService: {},
	ScheduleService: {},
	HttpTriggerService: {},
	WebsocketTriggerService: {},
	KafkaTriggerService: {},
	NatsTriggerService: {},
	PostgresTriggerService: {},
	MqttTriggerService: {},
	SqsTriggerService: {},
	GcpTriggerService: {},
	AzureTriggerService: {}
}))

vi.mock('$lib/utils', () => ({
	emptyString: (value: string | undefined | null) => !value,
	generateRandomString: () => 'generated_id'
}))

vi.mock('$lib/scripts', () => ({
	scriptLangToEditorLang: (language: string) => language
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: () => h.currentModel,
	getMetadataModel: () => h.currentModel,
	copilotInfo: {
		subscribe: (run: (value: unknown) => void) => {
			run({})
			return () => undefined
		}
	}
}))

vi.mock('@leeoniya/ufuzzy', () => ({
	default: class {
		search() {
			return [[], [], []]
		}
	}
}))

function streamOf(chunks: unknown[]): any {
	return (async function* () {
		for (const chunk of chunks) {
			yield chunk
		}
	})()
}

function textDelta(text: string) {
	return { type: 'content_block_delta', delta: { type: 'text_delta', text } }
}

const messages: ChatCompletionMessageParam[] = [{ role: 'user', content: 'hi' }]

let anthropicCreate: ReturnType<typeof vi.fn>
let anthropicStream: ReturnType<typeof vi.fn>
let openaiCreate: ReturnType<typeof vi.fn>

async function setupClients() {
	const { workspaceAIClients } = await import('./lib')

	anthropicCreate = vi.fn().mockResolvedValue({
		content: [
			{ type: 'text', text: 'Hel' },
			{ type: 'thinking', thinking: 'ignored' },
			{ type: 'text', text: 'lo' }
		]
	})
	anthropicStream = vi
		.fn()
		.mockReturnValue(
			streamOf([
				{ type: 'message_start' },
				textDelta('Hel'),
				{ type: 'content_block_delta', delta: { type: 'input_json_delta', partial_json: '{' } },
				textDelta('lo'),
				{ type: 'message_stop' }
			])
		)
	openaiCreate = vi.fn().mockResolvedValue({ choices: [{ message: { content: 'openai text' } }] })

	vi.spyOn(workspaceAIClients, 'getAnthropicClient').mockReturnValue({
		messages: { create: anthropicCreate, stream: anthropicStream }
	} as any)
	vi.spyOn(workspaceAIClients, 'getOpenaiClient').mockReturnValue({
		chat: { completions: { create: openaiCreate } }
	} as any)
}

beforeEach(async () => {
	await setupClients()
})

afterEach(() => {
	vi.restoreAllMocks()
	h.currentModel = undefined
})

describe('Anthropic Messages API routing', () => {
	it('getNonStreamingCompletion routes Foundry Claude through the Anthropic client', async () => {
		const { getNonStreamingCompletion } = await import('./lib')
		h.currentModel = { provider: 'azure_foundry', model: 'claude-sonnet-5' }

		const response = await getNonStreamingCompletion(messages, new AbortController())

		expect(anthropicCreate).toHaveBeenCalledTimes(1)
		expect(openaiCreate).not.toHaveBeenCalled()
		// text blocks concatenated, non-text blocks dropped
		expect(response).toBe('Hello')

		const headers = anthropicCreate.mock.calls[0][1].headers
		// X-Provider must carry the real provider so the backend resolves Foundry
		// credentials/URL; the SDK header selects the Messages API path.
		expect(headers['X-Provider']).toBe('azure_foundry')
		expect(headers['X-Anthropic-SDK']).toBe('true')
	})

	it('getNonStreamingCompletion routes native Anthropic through the Anthropic client', async () => {
		const { getNonStreamingCompletion } = await import('./lib')
		h.currentModel = { provider: 'anthropic', model: 'claude-opus-4-8' }

		await getNonStreamingCompletion(messages, new AbortController())

		expect(anthropicCreate).toHaveBeenCalledTimes(1)
		expect(anthropicCreate.mock.calls[0][1].headers['X-Provider']).toBe('anthropic')
	})

	it('getNonStreamingCompletion keeps non-Claude Foundry models on the OpenAI path', async () => {
		const { getNonStreamingCompletion } = await import('./lib')
		h.currentModel = { provider: 'azure_foundry', model: 'gpt-4o' }

		await getNonStreamingCompletion(messages, new AbortController())

		expect(anthropicCreate).not.toHaveBeenCalled()
		expect(openaiCreate).toHaveBeenCalledTimes(1)
	})

	it('getCompletion adapts the Anthropic stream into OpenAI text chunks', async () => {
		const { getCompletion, getResponseFromEvent } = await import('./lib')
		h.currentModel = { provider: 'azure_foundry', model: 'claude-sonnet-5' }

		const completion = await getCompletion(messages, new AbortController())

		let text = ''
		let chunks = 0
		for await (const part of completion) {
			chunks++
			text += getResponseFromEvent(part)
		}

		expect(anthropicStream).toHaveBeenCalledTimes(1)
		// only the two text deltas surface; message_start/stop and input_json are dropped
		expect(chunks).toBe(2)
		expect(text).toBe('Hello')
	})

	it('testKey routes Foundry Claude through the Anthropic client', async () => {
		const { testKey } = await import('./lib')

		await testKey({
			resourcePath: 'u/admin/foundry',
			model: 'claude-sonnet-5',
			abortController: new AbortController(),
			messages,
			aiProvider: 'azure_foundry'
		})

		expect(anthropicCreate).toHaveBeenCalledTimes(1)
		const headers = anthropicCreate.mock.calls[0][1].headers
		expect(headers['X-Provider']).toBe('azure_foundry')
		expect(headers['X-Resource-Path']).toBe('u/admin/foundry')
	})

	it('getFimCompletion no-ops for Anthropic Messages API models', async () => {
		const { getFimCompletion } = await import('./lib')
		const fetchSpy = vi.spyOn(globalThis, 'fetch')

		for (const provider of ['anthropic', 'azure_foundry'] as const) {
			const result = await getFimCompletion(
				'prefix',
				'suffix',
				{ provider, model: 'claude-sonnet-5' },
				new AbortController()
			)
			expect(result).toBeUndefined()
		}
		// no autocomplete request should be issued for these models
		expect(fetchSpy).not.toHaveBeenCalled()
	})
})
