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
let openaiResponsesCreate: ReturnType<typeof vi.fn>

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
	openaiResponsesCreate = vi.fn().mockResolvedValue({ output_text: 'responses text' })

	vi.spyOn(workspaceAIClients, 'getAnthropicClient').mockReturnValue({
		messages: { create: anthropicCreate, stream: anthropicStream }
	} as any)
	vi.spyOn(workspaceAIClients, 'getOpenaiClient').mockReturnValue({
		chat: { completions: { create: openaiCreate } },
		responses: { create: openaiResponsesCreate }
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
		// credentials and URL.
		expect(headers['X-Provider']).toBe('azure_foundry')
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

	it('raises the Claude output budget only from Opus 4.5 on', async () => {
		const { getModelMaxTokens } = await import('./lib')
		// 4.5+ matches Sonnet's budget...
		for (const model of ['claude-opus-4-5', 'claude-opus-4-8', 'claude-opus-5']) {
			expect(getModelMaxTokens('anthropic', model)).toBe(64000)
		}
		expect(getModelMaxTokens('openrouter', 'anthropic/claude-opus-4.5')).toBe(64000)
		expect(getModelMaxTokens('aws_bedrock', 'anthropic.claude-opus-4-5-20251101-v1:0')).toBe(64000)
		// ...while Opus 4.1 and older cap at 32K and must not be raised.
		expect(getModelMaxTokens('anthropic', 'claude-opus-4-1')).toBe(32000)
		expect(getModelMaxTokens('aws_bedrock', 'anthropic.claude-opus-4-1-20250805-v1:0')).toBe(32000)
		expect(getModelMaxTokens('aws_bedrock', 'anthropic.claude-opus-4-20250514-v1:0')).toBe(32000)
	})

	it('caps max_tokens for metadata completions so the Anthropic SDK stays non-streaming', async () => {
		const { getNonStreamingCompletion, getNonStreamingMetadataCompletion, METADATA_MAX_TOKENS } =
			await import('./lib')
		// claude-sonnet defaults to 64000 max_tokens; the Anthropic SDK refuses a
		// non-streaming request that large (>10min worst case), which silently broke
		// session auto-rename and the other metadata generators.
		h.currentModel = { provider: 'anthropic', model: 'claude-sonnet-4-6' }

		await getNonStreamingCompletion(messages, new AbortController())
		expect(anthropicCreate.mock.calls[0][0].max_tokens).toBe(64000)

		anthropicCreate.mockClear()
		await getNonStreamingMetadataCompletion(messages, new AbortController())
		expect(anthropicCreate.mock.calls[0][0].max_tokens).toBe(METADATA_MAX_TOKENS)
		expect(METADATA_MAX_TOKENS).toBeLessThanOrEqual(21333)
	})

	it('caps max_output_tokens for metadata completions on the OpenAI Responses path', async () => {
		const { getNonStreamingCompletion, getNonStreamingMetadataCompletion, METADATA_MAX_TOKENS } =
			await import('./lib')
		// OpenAI/Azure non-streaming routes through the Responses API; the cap must
		// reach it too, not just the Anthropic and chat.completions paths.
		h.currentModel = { provider: 'openai', model: 'gpt-4o' }

		await getNonStreamingCompletion(messages, new AbortController())
		expect(openaiResponsesCreate).toHaveBeenCalledTimes(1)
		expect(openaiResponsesCreate.mock.calls[0][0].max_output_tokens).toBe(16384)

		openaiResponsesCreate.mockClear()
		await getNonStreamingMetadataCompletion(messages, new AbortController())
		expect(openaiResponsesCreate.mock.calls[0][0].max_output_tokens).toBe(METADATA_MAX_TOKENS)
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

// Without an explicit breakpoint Anthropic charges the full prompt on every iteration,
// and the tool definitions alone are the largest part of a chat request. OpenRouter
// forwards the field only for Anthropic-backed models, hence the model-level gate.
describe('OpenRouter prompt caching', () => {
	const conversation: ChatCompletionMessageParam[] = [
		{ role: 'system', content: 'system prompt' },
		{ role: 'user', content: 'first' },
		{ role: 'assistant', content: 'answer' },
		{ role: 'tool', tool_call_id: 't1', content: 'tool output' }
	]

	// The `~` form is OpenRouter's own floating alias for the same vendor, so it has
	// to reach the same gate — a prefix match on the raw id silently misses it and
	// puts the chat back on full price every turn.
	it.each(['anthropic/claude-sonnet-5', '~anthropic/claude-sonnet-latest'])(
		'breaks on the system prompt and the newest user turn for %s',
		async (model) => {
			const { getCompletion } = await import('./lib')
			h.currentModel = { provider: 'openrouter', model }

			await getCompletion([...conversation], new AbortController(), undefined, {
				promptCaching: true
			})

			const sent = openaiCreate.mock.calls[0][0].messages
			const ephemeral = { type: 'ephemeral' }
			expect(sent[0].content).toEqual([
				{ type: 'text', text: 'system prompt', cache_control: ephemeral }
			])
			expect(sent[1].content).toEqual([{ type: 'text', text: 'first', cache_control: ephemeral }])
			expect(sent[3].content).toBe('tool output')
			// The chat replays this same history through the Anthropic path, which rejects
			// unknown fields on its own blocks, so the originals must come back untouched.
			expect(conversation[0].content).toBe('system prompt')
		}
	)

	it('leaves other OpenRouter upstreams alone', async () => {
		const { getCompletion } = await import('./lib')
		h.currentModel = { provider: 'openrouter', model: 'openai/gpt-5.1' }

		await getCompletion([...conversation], new AbortController(), undefined, {
			promptCaching: true
		})

		expect(openaiCreate.mock.calls[0][0].messages[0].content).toBe('system prompt')
	})
})
