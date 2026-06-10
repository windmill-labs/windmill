import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { runChatLoop, type ChatLoopConfig } from './chatLoop'
import type { ReasoningProviderModel } from '../reasoningRegistry'

const mocks = vi.hoisted(() => ({
	getCompletion: vi.fn(),
	parseOpenAICompletion: vi.fn(),
	providerSupportsWebSearch: vi.fn(),
	getOpenAIResponsesCompletion: vi.fn(),
	parseOpenAIResponsesCompletion: vi.fn(),
	getAnthropicCompletion: vi.fn(),
	parseAnthropicCompletion: vi.fn(),
	resolveEffectiveReasoning: vi.fn()
}))

vi.mock('../lib', () => ({
	getCompletion: mocks.getCompletion,
	parseOpenAICompletion: mocks.parseOpenAICompletion,
	providerSupportsWebSearch: mocks.providerSupportsWebSearch
}))

vi.mock('../reasoningRegistry', () => ({
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
	callbacks = createCallbacks()
}: {
	workspace: string
	modelProvider?: ReasoningProviderModel
	callbacks?: ChatLoopConfig['callbacks']
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
		maxIterations: 1
	}
}

describe('runChatLoop web search fallback', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		mocks.providerSupportsWebSearch.mockImplementation(
			(provider) => provider === 'openai' || provider === 'anthropic'
		)
		mocks.resolveEffectiveReasoning.mockReturnValue(undefined)
		mocks.parseOpenAIResponsesCompletion.mockResolvedValue({
			shouldContinue: false,
			tokenUsage
		})
	})

	it('retries once without OpenAI web search and caches unsupported provider models', async () => {
		const callbacks = createCallbacks()
		const workspace = `workspace-${crypto.randomUUID()}`

		mocks.getOpenAIResponsesCompletion
			.mockRejectedValueOnce(new Error('web_search is not supported for this model'))
			.mockResolvedValue({})

		await runChatLoop(createConfig({ workspace, callbacks }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(2)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[0][3]).toEqual(
			expect.objectContaining({ webSearch: true })
		)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[1][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
		expect(callbacks.setToolStatus).toHaveBeenCalledWith(
			expect.stringContaining('web_search_unavailable:'),
			expect.objectContaining({
				content: 'You can disable websearch in your workspace settings.',
				error: 'You can disable websearch in your workspace settings.',
				toolName: 'web_search'
			})
		)

		await runChatLoop(createConfig({ workspace }))

		expect(mocks.getOpenAIResponsesCompletion).toHaveBeenCalledTimes(3)
		expect(mocks.getOpenAIResponsesCompletion.mock.calls[2][3]).toEqual(
			expect.objectContaining({ webSearch: false })
		)
	})
})
