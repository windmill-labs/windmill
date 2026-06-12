import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { runChatLoop } from './chatLoop'

const mocks = vi.hoisted(() => ({
	getCurrentModel: vi.fn(),
	tryGetCurrentModel: vi.fn(),
	isWebSearchEnabledForProvider: vi.fn(),
	logAiChat: vi.fn(),
	sendUserToast: vi.fn(),
	getOpenaiClient: vi.fn(),
	getAnthropicClient: vi.fn(),
	runChatLoop: vi.fn()
}))

vi.mock('monaco-editor', () => ({
	Selection: class Selection {}
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		logAiChat: mocks.logAiChat
	},
	ScriptService: {},
	FlowService: {},
	JobService: {}
}))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined },
	userStore: {
		subscribe: (run: (value: { username: string }) => void) => {
			run({ username: 'admin' })
			return () => undefined
		}
	}
}))

vi.mock('$lib/toast', () => ({
	sendUserToast: mocks.sendUserToast
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: mocks.getCurrentModel,
	tryGetCurrentModel: mocks.tryGetCurrentModel,
	getCombinedCustomPrompt: () => '',
	isWebSearchEnabledForProvider: mocks.isWebSearchEnabledForProvider
}))

vi.mock('../lib', () => ({
	workspaceAIClients: {
		subscribe: () => () => undefined,
		getOpenaiClient: mocks.getOpenaiClient,
		getAnthropicClient: mocks.getAnthropicClient
	}
}))

vi.mock('./api/apiTools', () => ({
	loadApiTools: vi.fn()
}))

// Mock only runChatLoop; keep the real truncateToToolPairedPrefix (pure) that
// the manager uses to commit partial output.
vi.mock('./chatLoop', async (importOriginal) => ({
	...(await importOriginal<typeof import('./chatLoop')>()),
	runChatLoop: mocks.runChatLoop
}))

vi.mock('./global/gate', () => ({
	isGlobalAiEnabled: () => true
}))

// Force BROWSER=true so localStorage-backed autonomy persistence is exercised
// (the vitest "server" env reports BROWSER=false, which would short-circuit it).
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

beforeEach(() => {
	vi.clearAllMocks()
	mocks.getCurrentModel.mockReturnValue(undefined)
	mocks.tryGetCurrentModel.mockReturnValue(undefined)
	mocks.isWebSearchEnabledForProvider.mockReturnValue(true)
	mocks.logAiChat.mockResolvedValue(undefined)
	mocks.getOpenaiClient.mockReturnValue({})
	mocks.getAnthropicClient.mockReturnValue({})
	mocks.runChatLoop.mockResolvedValue({
		addedMessages: [],
		tokenUsage: { prompt: 0, completion: 0, total: 0 },
		hitMaxIterations: false
	})
})

function createFlowHelpers({
	hasPendingChanges,
	acceptAllModuleActions,
	testFlow = vi.fn()
}: {
	hasPendingChanges: () => boolean
	acceptAllModuleActions: () => void
	testFlow?: FlowAIChatHelpers['testFlow']
}): FlowAIChatHelpers {
	return {
		getFlowAndSelectedId: vi.fn(),
		getRootModules: vi.fn(),
		inlineScriptSession: { get: vi.fn(), set: vi.fn(), clear: vi.fn() },
		setSnapshot: vi.fn(),
		revertToSnapshot: vi.fn(),
		setCode: vi.fn(),
		setFlowJson: vi.fn(),
		getFlowInputsSchema: vi.fn(),
		updateExprsToSet: vi.fn(),
		acceptAllModuleActions,
		rejectAllModuleActions: vi.fn(),
		hasPendingChanges,
		selectStep: vi.fn(),
		testFlow,
		getLintErrors: vi.fn()
	} as unknown as FlowAIChatHelpers
}

describe('AIChatManager request errors', () => {
	const openaiModel = { provider: 'openai', model: 'gpt-4o' }

	beforeEach(() => {
		localStorage.clear()
		mocks.getCurrentModel.mockReturnValue(openaiModel)
		mocks.tryGetCurrentModel.mockReturnValue(openaiModel)
	})

	it('does not add a web-search hint to generic request errors', async () => {
		const manager = new AIChatManager()
		manager.instructions = 'Search for recent docs'
		mocks.isWebSearchEnabledForProvider.mockReturnValue(true)
		mocks.runChatLoop.mockRejectedValueOnce(new Error('provider quota exceeded'))

		await manager.sendRequest()

		expect(mocks.sendUserToast).toHaveBeenLastCalledWith(
			'Failed to send request: provider quota exceeded',
			true
		)
	})

	it('adds the web-search hint when fallback happened and the request still fails', async () => {
		const manager = new AIChatManager()
		manager.instructions = 'Search for recent docs'
		mocks.isWebSearchEnabledForProvider.mockReturnValue(true)
		mocks.runChatLoop.mockImplementationOnce(async (config) => {
			config.onWebSearchUnavailable?.()
			throw new Error('provider quota exceeded')
		})

		await manager.sendRequest()

		expect(mocks.sendUserToast).toHaveBeenLastCalledWith(
			'Failed to send request: provider quota exceeded. Web search is unavailable for this provider/model/key. Disable web search in workspace settings and try again.',
			true
		)
	})

	it('does not add the web search settings hint when web search is disabled', async () => {
		const manager = new AIChatManager()
		manager.instructions = 'Search for recent docs'
		mocks.isWebSearchEnabledForProvider.mockReturnValue(false)
		mocks.runChatLoop.mockRejectedValueOnce(new Error('provider quota exceeded'))

		await manager.sendRequest()

		expect(mocks.sendUserToast).toHaveBeenLastCalledWith(
			'Failed to send request: provider quota exceeded',
			true
		)
	})
})

describe('AIChatManager autonomy mode', () => {
	beforeEach(() => {
		localStorage.clear()
		// These tests exercise the transition into auto-accept, so start from the
		// ask-permission baseline rather than the new auto-accept-edits default.
		localStorage.setItem('ai-chat-autonomy-mode', AIAutonomyMode.DEFAULT)
		vi.clearAllMocks()
	})

	it('accepts pending flow edits when auto-accept is enabled from script mode', async () => {
		const manager = new AIChatManager()
		const acceptAllModuleActions = vi.fn()

		manager.mode = AIMode.SCRIPT
		manager.setFlowHelpers(
			createFlowHelpers({
				hasPendingChanges: () => true,
				acceptAllModuleActions
			})
		)

		manager.setAutonomyMode(AIAutonomyMode.ACCEPT_EDIT)

		expect(acceptAllModuleActions).toHaveBeenCalledTimes(1)
	})

	it('accepts pending flow edits when helpers register while auto-accept is already enabled', async () => {
		const manager = new AIChatManager()
		const acceptAllModuleActions = vi.fn()

		manager.mode = AIMode.SCRIPT
		manager.setAutonomyMode(AIAutonomyMode.ACCEPT_EDIT)
		manager.setFlowHelpers(
			createFlowHelpers({
				hasPendingChanges: () => true,
				acceptAllModuleActions
			})
		)

		expect(acceptAllModuleActions).toHaveBeenCalledTimes(1)
	})

	it('waits for flow step editor review before resolving applyScriptEditorCode', async () => {
		const manager = new AIChatManager()
		let finishReview: (() => void) | undefined
		const reviewPromise = new Promise<void>((resolve) => {
			finishReview = resolve
		})
		const hideDiffMode = vi.fn()
		const reviewAndApplyCode = vi.fn(() => reviewPromise)
		const opts = { mode: 'apply' } satisfies ReviewChangesOpts

		manager.listenForCurrentEditorChanges({
			type: 'script',
			stepId: 'step-a',
			editor: {
				reviewAndApplyCode,
				getLintErrors: vi.fn()
			},
			showDiffMode: vi.fn(),
			hideDiffMode,
			diffMode: false,
			lastDeployedCode: undefined
		} as unknown as CurrentEditor)

		let applied = false
		const applyPromise = manager
			.applyScriptEditorCode('export async function main() {}', opts)
			.then(() => {
				applied = true
			})

		await Promise.resolve()

		expect(hideDiffMode).toHaveBeenCalledTimes(1)
		expect(reviewAndApplyCode).toHaveBeenCalledWith('export async function main() {}', opts)
		expect(applied).toBe(false)

		finishReview?.()
		await applyPromise

		expect(applied).toBe(true)
	})

	it('does not pass the AI session id as a flow test conversation id in global mode', async () => {
		const manager = new AIChatManager()
		const testFlow = vi.fn(async () => 'job-flow-preview')

		manager.isSessionChat = true
		manager.sessionId = 'htc1xouxd96dcyo6ruqo39'
		manager.setFlowHelpers(
			createFlowHelpers({
				hasPendingChanges: () => false,
				acceptAllModuleActions: vi.fn(),
				testFlow
			})
		)

		manager.changeMode(AIMode.GLOBAL)
		const jobId = await manager.helpers.testActiveFlow({ name: 'Ada' })

		expect(jobId).toBe('job-flow-preview')
		expect(testFlow).toHaveBeenCalledWith({ name: 'Ada' })
	})
})

describe('AIChatManager persisted autonomy default', () => {
	// Mirrors the private storage keys in AIChatManager.svelte.ts.
	const AUTONOMY_KEY = 'ai-chat-autonomy-mode'
	const LEGACY_YOLO_KEY = 'ai-chat-yolo-mode'

	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
	})

	it('defaults to auto-accept edits when no preference is stored', () => {
		expect(new AIChatManager().autonomyMode).toBe(AIAutonomyMode.ACCEPT_EDIT)
	})

	it('maps the legacy auto-accept-tool-confirmations flag to YOLO', () => {
		localStorage.setItem(LEGACY_YOLO_KEY, 'true')
		expect(new AIChatManager().autonomyMode).toBe(AIAutonomyMode.YOLO)
	})

	it('restores an explicitly persisted autonomy mode', () => {
		localStorage.setItem(AUTONOMY_KEY, AIAutonomyMode.DEFAULT)
		expect(new AIChatManager().autonomyMode).toBe(AIAutonomyMode.DEFAULT)
	})
})

describe('AIChatManager context compaction', () => {
	// claude-sonnet-4-6 resolves to a known 1M window (modelConfig is
	// unmocked): compaction triggers at a projected 800k and drops head
	// messages until ~700k.
	const anthropicModel = { provider: 'anthropic', model: 'claude-sonnet-4-6' }

	// The turn-outcome handling rolls back turns with no usable output, so every
	// sendRequest here must produce a reply to take the clean-commit path.
	const replyWith = (
		reply: string,
		lastIterationUsage: { prompt: number; completion: number; total: number } | null = null
	) =>
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: reply }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: lastIterationUsage ?? { prompt: 0, completion: 0, total: 0 },
				lastIterationUsage,
				hitMaxIterations: false
			}
		})

	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
		mocks.getCurrentModel.mockReturnValue(anthropicModel)
		mocks.tryGetCurrentModel.mockReturnValue(anthropicModel)
		replyWith('done')
	})

	it('compacts the stored history before sending once reported usage projects over the trigger', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) }, // ~100k estimated tokens
			{ role: 'assistant', content: 'b'.repeat(400_000) }, // ~100k
			{ role: 'user', content: 'c'.repeat(400) },
			{ role: 'assistant', content: 'd'.repeat(400) }
		]
		// Provider fact: 850k used. Projected past the 800k trigger, so ~150k
		// must be freed to come back to the 700k target — the first user +
		// assistant pair (~200k estimated).
		manager.contextUsage = 850_000
		manager.instructions = 'next question'

		await manager.sendRequest()

		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		expect(sent.length).toBe(3)
		expect(sent[0]).toMatchObject({ role: 'user', content: 'c'.repeat(400) })
		// The mutation is on the stored history, not a per-send copy: the head
		// pair is gone for good and the turn's reply was committed on top
		expect(manager.messages.length).toBe(4)
		expect(manager.messages[0]).toMatchObject({ role: 'user', content: 'c'.repeat(400) })
		// The trigger fact is debited by the freed estimate until the next report
		expect(manager.contextUsage).toBe(650_000)
		// The display message for the sent prompt re-bases onto the compacted history
		const userDisplay = manager.displayMessages.find((m) => m.role === 'user')
		expect(userDisplay && 'index' in userDisplay ? userDisplay.index : undefined).toBe(2)
	})

	it('updates the reported usage after every send, including compacted ones', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) },
			{ role: 'user', content: 'c'.repeat(400) }
		]
		manager.contextUsage = 850_000
		manager.instructions = 'next question'
		replyWith('done', { prompt: 720_000, completion: 1_000, total: 721_000 })

		await manager.sendRequest()

		// The report describes exactly what was sent (the compacted history), so
		// it replaces the debited estimate wholesale.
		expect(manager.contextUsage).toBe(721_000)
	})

	it('does not compact when no usage has been reported yet', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) }
		]
		manager.contextUsage = undefined
		manager.instructions = 'next question'

		await manager.sendRequest()

		expect(mocks.runChatLoop.mock.calls[0][0].messages.length).toBe(3)
		// and no report from the loop leaves the fact unset
		expect(manager.contextUsage).toBeUndefined()
	})

	it('does not compact when the model context window is unknown', async () => {
		mocks.getCurrentModel.mockReturnValue({ provider: 'custom', model: 'mystery-model-9000' })
		mocks.tryGetCurrentModel.mockReturnValue({ provider: 'custom', model: 'mystery-model-9000' })
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) }
		]
		manager.contextUsage = 10_000_000
		manager.instructions = 'next question'

		await manager.sendRequest()

		expect(mocks.runChatLoop.mock.calls[0][0].messages.length).toBe(3)
	})

	it('never drops the most recent message', () => {
		const manager = new AIChatManager()
		manager.messages = [{ role: 'user', content: 'a'.repeat(400_000) }]
		expect(manager.compactOldestMessages(Number.MAX_SAFE_INTEGER)).toBe(0)
		expect(manager.messages.length).toBe(1)
	})

	it('keeps dropping past dangling turns so the history restarts on a user message', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{
				role: 'assistant',
				content: 'calling tools',
				tool_calls: [
					{ id: '1', type: 'function', function: { name: 'x', arguments: '{}' } },
					{ id: '2', type: 'function', function: { name: 'y', arguments: '{}' } }
				]
			},
			{ role: 'tool', content: 'result 1', tool_call_id: '1' },
			{ role: 'tool', content: 'result 2', tool_call_id: '2' },
			{ role: 'user', content: 'follow-up' },
			{ role: 'user', content: 'latest' }
		]
		// Freeing 1 token is satisfied by the first drop alone, but the tool
		// results would dangle without their assistant tool_calls message
		manager.compactOldestMessages(1)
		expect(manager.messages.map((m) => m.role)).toEqual(['user', 'user'])
	})

	it('re-bases display message indices and clamps fully-compacted ones to 0', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400) }, // ~100 estimated tokens
			{ role: 'assistant', content: 'b'.repeat(400) }, // ~100
			{ role: 'user', content: 'c' },
			{ role: 'user', content: 'd' }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'first', index: 0 },
			{ role: 'assistant', content: 'answer' },
			{ role: 'user', content: 'second', index: 2 },
			{ role: 'user', content: 'third', index: 3 }
		]
		manager.compactOldestMessages(150)
		expect(manager.messages.map((m) => m.content)).toEqual(['c', 'd'])
		expect(manager.displayMessages.map((m) => ('index' in m ? m.index : undefined))).toEqual([
			0,
			undefined,
			0,
			1
		])
	})

	it('clears the reported usage when history is rewound', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'q1' },
			{ role: 'assistant', content: 'a1' }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'q1', index: 0 },
			{ role: 'assistant', content: 'a1' }
		]
		manager.contextUsage = 5000
		manager.restartGeneration(0)
		expect(manager.contextUsage).toBeUndefined()
	})

	it('clears the reported usage when saveAndClear resets the conversation', async () => {
		const manager = new AIChatManager()
		manager.contextUsage = 1000
		await manager.saveAndClear()
		expect(manager.contextUsage).toBeUndefined()
	})
})

const assistantToolCall = (id: string): ChatCompletionMessageParam => ({
	role: 'assistant',
	content: '',
	tool_calls: [{ id, type: 'function', function: { name: 'do_thing', arguments: '{}' } }]
})
const toolResult = (id: string): ChatCompletionMessageParam => ({
	role: 'tool',
	tool_call_id: id,
	content: 'ok'
})

describe('AIChatManager sendRequest lifecycle', () => {
	beforeEach(() => {
		localStorage.clear()
		// The send path reads the current model (request logging + context window
		// lookup), so it must be a real object (the file-level beforeEach defaults
		// it to undefined). 'test-model' has no known window → compaction stays off.
		mocks.getCurrentModel.mockReturnValue({ model: 'test-model', provider: 'openai' })
	})

	it('restores the message to the composer when the model returns no output (#2)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const restoreInstructions = vi.fn()
		manager.setAiChatInput({ restoreInstructions, focusInput: vi.fn() } as any)

		// Empty turn: the loop produces no messages and no display output.
		vi.mocked(runChatLoop).mockResolvedValue({
			addedMessages: [],
			tokenUsage: {} as any,
			lastIterationUsage: null,
			hitMaxIterations: false
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		// The empty user turn is rolled back out of the transcript...
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(false)
		expect(manager.messages.some((m) => m.role === 'user')).toBe(false)
		// ...and its text is handed back to the composer.
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [])
		expect(manager.loading).toBe(false)
	})

	it('restores the message when a completed turn produced only reasoning (#2)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const restoreInstructions = vi.fn()
		manager.setAiChatInput({ restoreInstructions, focusInput: vi.fn() } as any)

		// The model finishes (no abort, no error) having emitted only reasoning —
		// nothing replayable as context, so the turn is as unsent as an empty one.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onReasoningStart?.()
			config.callbacks.onReasoningDelta?.('hmm...')
			config.callbacks.onMessageEnd()
			return {
				addedMessages: [],
				tokenUsage: {} as any,
				lastIterationUsage: null,
				hitMaxIterations: false
			}
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		expect(manager.displayMessages).toHaveLength(0)
		expect(manager.messages.some((m) => m.role === 'user')).toBe(false)
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [])
		expect(manager.loading).toBe(false)
	})

	it('does NOT restore on a normal turn that produced output', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const restoreInstructions = vi.fn()
		manager.setAiChatInput({ restoreInstructions, focusInput: vi.fn() } as any)

		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('hello')
			config.callbacks.onMessageEnd()
			return {
				addedMessages: [],
				tokenUsage: {} as any,
				lastIterationUsage: null,
				hitMaxIterations: false
			}
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		expect(restoreInstructions).not.toHaveBeenCalled()
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(true)
		expect(manager.displayMessages.some((m) => m.role === 'assistant')).toBe(true)
	})

	it('keeps the tool-paired prefix of a failed turn as context, dropping the dangling call (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ restoreInstructions: vi.fn(), focusInput: vi.fn() } as any)

		// Completed round-trip for 'a', then started 'b' and failed before its result.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.addedMessages!.push(assistantToolCall('a'), toolResult('a'), assistantToolCall('b'))
			throw new Error('boom')
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		// 'a' round-trip retained as context; dangling 'b' dropped.
		const toolMsgs = manager.messages.filter((m) => m.role === 'tool')
		expect(toolMsgs).toHaveLength(1)
		const hasDanglingB = manager.messages.some(
			(m) => m.role === 'assistant' && (m as any).tool_calls?.some((c: any) => c.id === 'b')
		)
		expect(hasDanglingB).toBe(false)
		// The user message is flagged so the Retry affordance shows.
		const lastUser = [...manager.displayMessages].reverse().find((m) => m.role === 'user')
		expect((lastUser as any)?.error).toBe(true)
		expect(manager.loading).toBe(false)
	})

	it('retains the partial answer text when cancelled mid-response, so a follow-up continues (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ restoreInstructions: vi.fn(), focusInput: vi.fn() } as any)

		// Model wrote part of an answer, then the user hit Stop (abort).
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('Here is the partial ')
			config.callbacks.onNewToken('answer')
			config.abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.instructions = 'write a long thing'
		await manager.sendRequest()

		// The partial answer is carried as context for the next message.
		const assistant = manager.messages.find((m) => m.role === 'assistant')
		expect(assistant?.content).toBe('Here is the partial answer')
		// And it stays visible in the transcript.
		expect(manager.displayMessages.some((m) => m.role === 'assistant')).toBe(true)
		expect(manager.loading).toBe(false)
	})

	it('restores the message and clears the reasoning bubble when cancelled while only thinking (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const restoreInstructions = vi.fn()
		manager.setAiChatInput({ restoreInstructions, focusInput: vi.fn() } as any)

		// Model was still thinking (no answer text) when the user hit Stop.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onReasoningStart?.()
			config.callbacks.onReasoningDelta?.('still thinking...')
			config.abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.instructions = 'think hard'
		await manager.sendRequest()

		// Nothing usable was produced → treat as unsent: roll the turn back out
		// (user message + stuck-open reasoning bubble) and restore the composer.
		expect(manager.messages.some((m) => m.role === 'assistant')).toBe(false)
		expect(manager.displayMessages.some((m) => m.role === 'assistant')).toBe(false)
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(false)
		expect(restoreInstructions).toHaveBeenCalledWith('think hard', [])
		expect(manager.loading).toBe(false)
	})

	it('keeps text flushed before a tool call when cancelled during the tool call (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const restoreInstructions = vi.fn()
		manager.setAiChatInput({ restoreInstructions, focusInput: vi.fn() } as any)

		// When a tool call starts streaming after some answer text, the parsers
		// flush onMessageEnd early (capturing the text and resetting currentReply)
		// while the structured message carrying that text is only pushed at clean
		// stream end. If the user cancels during the tool call, chatRequest's
		// catch calls onMessageEnd again with an empty currentReply — the captured
		// text must survive that second call.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('Partial from Claude')
			config.callbacks.onMessageEnd()
			config.abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.instructions = 'write a long thing'
		await manager.sendRequest()

		const assistant = manager.messages.find((m) => m.role === 'assistant')
		expect(assistant?.content).toBe('Partial from Claude')
		expect(restoreInstructions).not.toHaveBeenCalled()
		expect(manager.loading).toBe(false)
	})

	it('does not duplicate an already-committed answer when cancelled right after a completed message', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ restoreInstructions: vi.fn(), focusInput: vi.fn() } as any)

		// A message completed cleanly (structured message in addedMessages,
		// partialReply captured at its onMessageEnd), then the abort lands before
		// the next iteration produced anything — the stale partialReply must not
		// be committed a second time.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('The full answer')
			config.addedMessages!.push({ role: 'assistant', content: 'The full answer' })
			config.callbacks.onMessageEnd()
			config.abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		const assistants = manager.messages.filter((m) => m.role === 'assistant')
		expect(assistants).toHaveLength(1)
		expect(assistants[0]?.content).toBe('The full answer')
		expect(manager.loading).toBe(false)
	})

	it('does not re-commit the turn when a post-commit save throws', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ restoreInstructions: vi.fn(), focusInput: vi.fn() } as any)

		// Clean turn, but persisting it fails — the catch must not treat that as a
		// failed request and commit the collected messages a second time.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('hello')
			config.addedMessages!.push({ role: 'assistant', content: 'hello' })
			config.callbacks.onMessageEnd()
			return {
				addedMessages: config.addedMessages!,
				tokenUsage: {} as any,
				lastIterationUsage: null,
				hitMaxIterations: false
			}
		})
		const saveChat = vi
			.spyOn(manager.historyManager, 'saveChat')
			.mockResolvedValueOnce(undefined) // save right after the user message
			.mockRejectedValueOnce(new Error('persist failed')) // post-commit save

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		expect(saveChat).toHaveBeenCalledTimes(2)
		const assistants = manager.messages.filter((m) => m.role === 'assistant')
		expect(assistants).toHaveLength(1)
		// The request itself succeeded, so the user message is not flagged.
		const lastUser = [...manager.displayMessages].reverse().find((m) => m.role === 'user')
		expect((lastUser as any)?.error).toBeUndefined()
		expect(manager.loading).toBe(false)
	})

	it('removes the persisted chat when a rolled-back first turn empties the transcript', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ restoreInstructions: vi.fn(), focusInput: vi.fn() } as any)

		// saveChat no-ops on an empty transcript, so rolling back the only turn
		// must delete the chat entry persisted earlier in the turn instead.
		vi.mocked(runChatLoop).mockResolvedValue({
			addedMessages: [],
			tokenUsage: {} as any,
			lastIterationUsage: null,
			hitMaxIterations: false
		})
		const deletePastChat = vi.spyOn(manager.historyManager, 'deletePastChat')

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		expect(manager.displayMessages).toHaveLength(0)
		expect(deletePastChat).toHaveBeenCalledWith(manager.historyManager.getCurrentChatId())
		expect(manager.loading).toBe(false)
	})
})
