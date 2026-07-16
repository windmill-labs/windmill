import { beforeEach, describe, expect, it, vi } from 'vitest'
import { randomUUID } from '$lib/utils/uuid'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { runChatLoop, truncateToToolPairedPrefix, type ChatLoopConfig } from './chatLoop'
import type { ReasoningProviderModel } from '../reasoningRegistry'

const mocks = vi.hoisted(() => ({
	getCompletion: vi.fn(),
	parseOpenAICompletion: vi.fn(),
	providerSupportsWebSearch: vi.fn(),
	getOpenAIResponsesCompletion: vi.fn(),
	parseOpenAIResponsesCompletion: vi.fn(),
	getAnthropicCompletion: vi.fn(),
	parseAnthropicCompletion: vi.fn(),
	resolveRequestReasoning: vi.fn(),
	resolveEffectiveReasoning: vi.fn()
}))

vi.mock('../lib', () => ({
	getCompletion: mocks.getCompletion,
	parseOpenAICompletion: mocks.parseOpenAICompletion,
	providerSupportsWebSearch: mocks.providerSupportsWebSearch
}))

vi.mock('../reasoningRegistry', () => ({
	resolveRequestReasoning: mocks.resolveRequestReasoning,
	resolveEffectiveReasoning: mocks.resolveEffectiveReasoning
}))

vi.mock('./openai-responses', () => ({
	getOpenAIResponsesCompletion: mocks.getOpenAIResponsesCompletion,
	parseOpenAIResponsesCompletion: mocks.parseOpenAIResponsesCompletion
}))

vi.mock('./anthropic', () => ({
	getAnthropicCompletion: mocks.getAnthropicCompletion,
	parseAnthropicCompletion: mocks.parseAnthropicCompletion
}))

const tokenUsage = { prompt: 0, completion: 0, total: 0 }

function createCallbacks(): ChatLoopConfig['callbacks'] {
	return {
		onNewToken: vi.fn(),
		onMessageEnd: vi.fn(),
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	}
}

function createConfig({
	workspace,
	modelProvider = { provider: 'openai', model: 'gpt-4.1' },
	callbacks = createCallbacks(),
	onWebSearchUnavailable,
	onReasoningSummaryUnavailable
}: {
	workspace: string
	modelProvider?: ReasoningProviderModel
	callbacks?: ChatLoopConfig['callbacks']
	onWebSearchUnavailable?: ChatLoopConfig['onWebSearchUnavailable']
	onReasoningSummaryUnavailable?: ChatLoopConfig['onReasoningSummaryUnavailable']
}): ChatLoopConfig {
	const messages: ChatCompletionMessageParam[] = [{ role: 'user', content: 'search this' }]

	return {
		messages,
		systemMessage: { role: 'system', content: '' },
		tools: [],
		helpers: undefined,
		abortController: new AbortController(),
		callbacks,
		modelProvider,
		clients: {
			openai: {} as ChatLoopConfig['clients']['openai'],
			anthropic: {} as ChatLoopConfig['clients']['anthropic']
		},
		workspace,
		maxIterations: 1,
		onWebSearchUnavailable,
		onReasoningSummaryUnavailable
	}
}

describe('runChatLoop web search fallback', () => {
	beforeEach(() => {
		vi.resetAllMocks()
		mocks.providerSupportsWebSearch.mockImplementation(
			(provider) => provider === 'openai' || provider === 'anthropic'
		)
		mocks.resolveRequestReasoning.mockReturnValue(undefined)
		mocks.parseOpenAICompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
		mocks.parseAnthropicCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
	})

	it('retries once without OpenAI web search and caches unsupported provider models', async () => {
		const callbacks = createCallbacks()
		const onWebSearchUnavailable = vi.fn()
		const workspace = `workspace-${randomUUID()}`

		mocks.getOpenAIResponsesCompletion
			.mockRejectedValueOnce(
				Object.assign(new Error("Hosted tool 'web_search' is not supported with this model"), {
					status: 400,
					error: { type: 'invalid_request_error' }
				})
			)
			.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks, onWebSearchUnavailable }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
		expect(onWebSearchUnavailable).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()

		await runChatLoop(createConfig({ workspace }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(3)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[2][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
	})

	it('does not treat unrelated OpenAI tool errors as web search failures', async () => {
		const callbacks = createCallbacks()
		const workspace = `workspace-${randomUUID()}`

		mocks.getOpenAIResponsesCompletion.mockRejectedValueOnce(
			new Error('Unknown tool call: lookup_customer')
		)
		mocks.getCompletion.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(1)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
		expect(mocks.getCompletion).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()

		await runChatLoop(createConfig({ workspace }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
	})

	it('treats OpenAI hosted-tool incompatibility as web search unavailable', async () => {
		const callbacks = createCallbacks()
		const onWebSearchUnavailable = vi.fn()
		const workspace = `workspace-${randomUUID()}`

		mocks.getOpenAIResponsesCompletion
			.mockRejectedValueOnce(
				Object.assign(new Error('Hosted tools are not supported with this model'), {
					status: 400,
					error: { type: 'invalid_request_error' }
				})
			)
			.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks, onWebSearchUnavailable }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
		expect(onWebSearchUnavailable).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()
	})

	it('does not cache malformed web search requests as unavailable', async () => {
		const callbacks = createCallbacks()
		const workspace = `workspace-${randomUUID()}`

		mocks.getOpenAIResponsesCompletion.mockRejectedValueOnce(
			Object.assign(new Error('Invalid value for web_search.search_context_size'), {
				status: 400,
				error: { type: 'invalid_request_error' }
			})
		)
		mocks.getCompletion.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(1)
		expect(mocks.getCompletion).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()

		await runChatLoop(createConfig({ workspace }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
	})

	it('does not cache web-search rate limits as unavailable', async () => {
		const callbacks = createCallbacks()
		const workspace = `workspace-${randomUUID()}`

		mocks.getOpenAIResponsesCompletion.mockRejectedValueOnce(
			Object.assign(new Error('Rate limit exceeded for web_search'), {
				status: 429,
				error: { type: 'rate_limit_error' }
			})
		)
		mocks.getCompletion.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(1)
		expect(mocks.getCompletion).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()

		await runChatLoop(createConfig({ workspace }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
	})

	it('retries once without Anthropic web search when the model rejects it', async () => {
		const callbacks = createCallbacks()
		const onWebSearchUnavailable = vi.fn()
		const workspace = `workspace-${randomUUID()}`
		const modelProvider: ReasoningProviderModel = {
			provider: 'anthropic',
			model: 'claude-3-5-sonnet-latest'
		}

		mocks.getAnthropicCompletion
			.mockRejectedValueOnce(
				Object.assign(new Error('Your organization must enable web search in the Claude Console'), {
					status: 400,
					error: { type: 'invalid_request_error' }
				})
			)
			.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks, modelProvider, onWebSearchUnavailable }))

		expect(mocks.getAnthropicCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getAnthropicCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
		expect(mocks.getAnthropicCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
		expect(onWebSearchUnavailable).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()
	})

	it('does not retry Anthropic request errors that only mention web search input', async () => {
		const callbacks = createCallbacks()
		const workspace = `workspace-${randomUUID()}`
		const modelProvider: ReasoningProviderModel = {
			provider: 'anthropic',
			model: 'claude-sonnet-4-6'
		}
		const error = Object.assign(new Error('Invalid web_search query parameter'), {
			status: 400,
			error: { type: 'invalid_request_error' }
		})

		mocks.getAnthropicCompletion.mockRejectedValueOnce(error)

		await expect(runChatLoop(createConfig({ workspace, callbacks, modelProvider }))).rejects.toBe(
			error
		)

		expect(mocks.getAnthropicCompletion).toHaveBeenCalledTimes(1)
		expect(callbacks.setToolStatus).not.toHaveBeenCalled()
	})
})

describe('runChatLoop reasoning summary fallback', () => {
	beforeEach(() => {
		vi.resetAllMocks()
		mocks.providerSupportsWebSearch.mockReturnValue(false)
		mocks.resolveRequestReasoning.mockReturnValue('high')
		mocks.resolveEffectiveReasoning.mockReturnValue('high')
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
	})

	it('retries once without the summary on the unverified-org error and caches per workspace/provider', async () => {
		const onReasoningSummaryUnavailable = vi.fn()
		const workspace = `workspace-${randomUUID()}`
		const modelProvider: ReasoningProviderModel = { provider: 'openai', model: 'gpt-5.1' }

		mocks.getOpenAIResponsesCompletion
			.mockRejectedValueOnce(
				Object.assign(
					new Error('Your organization must be verified to generate reasoning summaries.'),
					{
						status: 400,
						param: 'reasoning.summary',
						code: 'unsupported_value',
						error: { type: 'invalid_request_error' }
					}
				)
			)
			.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, modelProvider, onReasoningSummaryUnavailable }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ reasoningSummary: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ reasoningSummary: false })
		)
		expect(onReasoningSummaryUnavailable).toHaveBeenCalledTimes(1)
		expect(mocks.getCompletion).not.toHaveBeenCalled()

		// Cached: a later run in the same workspace skips the summary outright,
		// but a different model on the same provider shares the cache entry.
		await runChatLoop(
			createConfig({
				workspace,
				modelProvider: { provider: 'openai', model: 'o3' }
			})
		)

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(3)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[2][3]).toEqual(
			expect.objectContaining({ reasoningSummary: false })
		)
	})

	it('composes with the web-search fallback when the errors arrive web-search first', async () => {
		mocks.providerSupportsWebSearch.mockReturnValue(true)
		const onReasoningSummaryUnavailable = vi.fn()
		const onWebSearchUnavailable = vi.fn()
		const workspace = `workspace-${randomUUID()}`
		const modelProvider: ReasoningProviderModel = { provider: 'openai', model: 'gpt-5.1' }

		mocks.getOpenAIResponsesCompletion
			.mockRejectedValueOnce(
				Object.assign(new Error("Hosted tool 'web_search' is not supported with this model"), {
					status: 400,
					error: { type: 'invalid_request_error' }
				})
			)
			.mockRejectedValueOnce(
				Object.assign(
					new Error('Your organization must be verified to generate reasoning summaries.'),
					{
						status: 400,
						param: 'reasoning.summary',
						code: 'unsupported_value',
						error: { type: 'invalid_request_error' }
					}
				)
			)
			.mockResolvedValue({})

		await runChatLoop(
			createConfig({
				workspace,
				modelProvider,
				onReasoningSummaryUnavailable,
				onWebSearchUnavailable
			})
		)

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(3)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true, reasoningSummary: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: false, reasoningSummary: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[2][3]).toEqual(
			expect.objectContaining({ webSearch: false, reasoningSummary: false })
		)
		expect(onWebSearchUnavailable).toHaveBeenCalledTimes(1)
		expect(onReasoningSummaryUnavailable).toHaveBeenCalledTimes(1)
		expect(mocks.getCompletion).not.toHaveBeenCalled()
	})

	it('does not request a summary when reasoning is explicitly off via a disable token', async () => {
		// gpt-5.1+ reasoning is turned off with the explicit 'none' effort on the
		// wire, while the effective reasoning resolves to undefined.
		mocks.resolveRequestReasoning.mockReturnValue('none')
		mocks.resolveEffectiveReasoning.mockReturnValue(undefined)
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		const workspace = `workspace-${randomUUID()}`

		await runChatLoop(
			createConfig({ workspace, modelProvider: { provider: 'openai', model: 'gpt-5.1' } })
		)

		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ reasoningEffort: 'none', reasoningSummary: false })
		)
	})
})

describe('runChatLoop lastIterationUsage', () => {
	beforeEach(() => {
		vi.resetAllMocks()
		mocks.resolveRequestReasoning.mockReturnValue(undefined)
	})

	it('keeps the usage of the last completion that reported it', async () => {
		const workspace = `workspace-${randomUUID()}`
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		mocks.parseOpenAIResponsesCompletion
			.mockResolvedValueOnce({
				shouldContinue: true,
				tokenUsage: { prompt: 1000, completion: 50, total: 1050 }
			})
			.mockResolvedValueOnce({
				shouldContinue: false,
				tokenUsage: { prompt: 1200, completion: 80, total: 1280 }
			})

		const result = await runChatLoop({ ...createConfig({ workspace }), maxIterations: 2 })

		expect(result.lastIterationUsage).toEqual({ prompt: 1200, completion: 80, total: 1280 })
		// the aggregate keeps summing across iterations
		expect(result.tokenUsage).toEqual({ prompt: 2200, completion: 130, total: 2330 })
	})

	it('ignores empty usage reports and returns null when none are real', async () => {
		const workspace = `workspace-${randomUUID()}`
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage: { prompt: 0, completion: 0, total: 0 }
		})

		const result = await runChatLoop(createConfig({ workspace }))

		expect(result.lastIterationUsage).toBeNull()
	})
})

// Builders for the message shapes the chat loop accumulates.
const assistant = (content: string): ChatCompletionMessageParam => ({ role: 'assistant', content })
const assistantTools = (...ids: string[]): ChatCompletionMessageParam => ({
	role: 'assistant',
	content: '',
	tool_calls: ids.map((id) => ({
		id,
		type: 'function',
		function: { name: 'do_thing', arguments: '{}' }
	}))
})
const tool = (id: string): ChatCompletionMessageParam => ({
	role: 'tool',
	tool_call_id: id,
	content: 'result'
})
const user = (content: string): ChatCompletionMessageParam => ({ role: 'user', content })

describe('truncateToToolPairedPrefix', () => {
	it('returns an empty array unchanged', () => {
		expect(truncateToToolPairedPrefix([])).toEqual([])
	})

	it('drops a trailing dangling tool_call (aborted before the result)', () => {
		const msgs = [assistantTools('a')]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([])
	})

	it('drops a partially-answered batch entirely (A answered, B missing)', () => {
		const msgs = [assistantTools('a', 'b'), tool('a')]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([])
	})

	it('keeps text + a completed round-trip, then drops the dangling tail', () => {
		const msgs = [
			assistant('let me check'),
			assistantTools('a'),
			tool('a'),
			assistant('more'),
			assistantTools('b') // dangling
		]
		expect(truncateToToolPairedPrefix(msgs)).toEqual([
			assistant('let me check'),
			assistantTools('a'),
			tool('a'),
			assistant('more')
		])
	})

	it('treats a user message as a valid boundary only when no tool calls are pending', () => {
		const ok = [assistantTools('a'), tool('a'), user('next')]
		expect(truncateToToolPairedPrefix(ok)).toEqual(ok)

		const dangling = [assistantTools('a'), user('next')]
		expect(truncateToToolPairedPrefix(dangling)).toEqual([])
	})

	it('leaves a valid full conversation unchanged (no loss on the normal path)', () => {
		const msgs = [
			assistant('thinking'),
			assistantTools('a', 'b'),
			tool('a'),
			tool('b'),
			assistant('done')
		]
		expect(truncateToToolPairedPrefix(msgs)).toEqual(msgs)
	})
})

describe('runChatLoop history sanitization', () => {
	beforeEach(() => {
		vi.resetAllMocks()
		mocks.providerSupportsWebSearch.mockReturnValue(false)
		mocks.resolveRequestReasoning.mockReturnValue(undefined)
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
	})

	it('replaces unparseable historical tool_call arguments before sending, without mutating the stored history', async () => {
		const config = createConfig({ workspace: `workspace-${randomUUID()}` })
		const poisoned: ChatCompletionMessageParam = {
			role: 'assistant',
			tool_calls: [
				{
					id: 'call_1',
					type: 'function',
					function: { name: 'patch_app_file', arguments: '{"path": "u/x", "old_string": "trunc' }
				}
			]
		}
		config.messages.push(poisoned, {
			role: 'tool',
			tool_call_id: 'call_1',
			content: 'Error while calling tool'
		})

		await runChatLoop(config)

		const sent = mocks.getOpenAIResponsesCompletion.mock.calls[0][0] as any[]
		const sentAssistant = sent.find((m) => m.role === 'assistant' && m.tool_calls)
		expect(sentAssistant.tool_calls[0].function.arguments).toBe('{}')
		expect((poisoned as any).tool_calls[0].function.arguments).toContain('trunc')
	})
})

describe('runChatLoop per-iteration vision gating', () => {
	beforeEach(() => {
		vi.resetAllMocks()
		mocks.providerSupportsWebSearch.mockReturnValue(false)
		mocks.resolveRequestReasoning.mockReturnValue(undefined)
	})

	// The loop owns the vision strip entirely — the caller passes the full
	// history even for a known text-only model (see AIChatManager.chatRequest).
	it('strips image parts from the first iteration on a known text-only model', async () => {
		const config = createConfig({
			workspace: `workspace-${randomUUID()}`,
			modelProvider: { provider: 'groq', model: 'llama-3.3-70b-versatile' }
		})
		config.messages.splice(0, config.messages.length, {
			role: 'user',
			content: [
				{ type: 'text', text: 'earlier turn' },
				{ type: 'image_url', image_url: { url: 'data:image/png;base64,IMG' } }
			]
		} as any)
		mocks.getCompletion.mockResolvedValue({})
		mocks.parseOpenAICompletion.mockResolvedValue({ shouldContinue: false, tokenUsage })

		await runChatLoop(config)

		expect(mocks.getCompletion).toHaveBeenCalled()
		expect(JSON.stringify(mocks.getCompletion.mock.calls[0][0])).not.toContain('image_url')
	})

	// The model selector stays enabled while the loop runs, and the loop re-reads
	// the model each iteration. The vision gate has to be re-applied at the same
	// cadence: filtering once at send start would ship the history's image parts
	// to a text-only model the user switched to mid-turn.
	it('strips image parts when the model switches to a text-only one mid-loop', async () => {
		const config = createConfig({ workspace: `workspace-${randomUUID()}` })
		config.maxIterations = 2
		config.messages.splice(0, config.messages.length, {
			role: 'user',
			content: [
				{ type: 'text', text: 'look at this' },
				{ type: 'image_url', image_url: { url: 'data:image/png;base64,IMG' } }
			]
		} as any)
		// vision model on the first iteration, known text-only on the second
		let iteration = 0
		Object.defineProperty(config, 'modelProvider', {
			get: () =>
				iteration === 0
					? { provider: 'openai', model: 'gpt-4.1' }
					: { provider: 'groq', model: 'llama-3.3-70b-versatile' }
		})
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		mocks.parseOpenAIResponsesCompletion.mockImplementation(async () => {
			iteration++
			return { shouldContinue: true, tokenUsage }
		})
		mocks.getCompletion.mockResolvedValue({})
		mocks.parseOpenAICompletion.mockResolvedValue({ shouldContinue: false, tokenUsage })

		await runChatLoop(config)

		// the vision iteration carries the image...
		const first = mocks.getOpenAIResponsesCompletion.mock.calls[0][0]
		expect(JSON.stringify(first)).toContain('image_url')
		// ...the text-only iteration must not
		expect(mocks.getCompletion).toHaveBeenCalled()
		const second = mocks.getCompletion.mock.calls[0][0]
		expect(JSON.stringify(second)).not.toContain('image_url')
	})

	// A history whose images together exceed the provider request-size limit gets
	// the whole request rejected with a 413 the vision-rejection fallback cannot
	// classify — the loop must keep the outbound copy under the byte cap.
	it('drops the oldest images when the history exceeds the total byte cap', async () => {
		const config = createConfig({ workspace: `workspace-${randomUUID()}` })
		const bigImage = () => ({
			type: 'image_url',
			// two of these exceed MAX_TOTAL_IMAGE_BYTES (12MB decoded)
			image_url: { url: 'data:image/png;base64,' + 'A'.repeat(9_000_000) }
		})
		config.messages.splice(
			0,
			config.messages.length,
			{ role: 'user', content: [{ type: 'text', text: 'old' }, bigImage()] } as any,
			{ role: 'assistant', content: 'ok' },
			{ role: 'user', content: [{ type: 'text', text: 'new' }, bigImage()] } as any
		)
		mocks.getOpenAIResponsesCompletion.mockResolvedValue({})
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({ shouldContinue: false, tokenUsage })

		await runChatLoop(config)

		const sent = mocks.getOpenAIResponsesCompletion.mock.calls[0][0] as any[]
		const users = sent.filter((m) => m.role === 'user')
		// the oldest message's image is stripped to a placeholder...
		expect(users[0].content).toBe('old\n[image omitted]')
		// ...the newest keeps its image part
		expect(users[1].content.some((p: any) => p.type === 'image_url')).toBe(true)
		// the stored history is untouched
		expect((config.messages[0] as any).content.some((p: any) => p.type === 'image_url')).toBe(true)
	})
})
