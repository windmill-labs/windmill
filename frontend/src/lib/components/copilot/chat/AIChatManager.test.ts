import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { runChatLoop } from './chatLoop'

vi.mock('monaco-editor', () => ({
	Selection: class Selection {}
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: {},
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
	sendUserToast: vi.fn()
}))

vi.mock('$lib/aiStore', () => ({
	// checkTokenUsageOverLimit reads getCurrentModel().model, so it must be a
	// real object. tryGetCurrentModel stays undefined so sendRequest skips the
	// logAiChat call (which would hit the empty WorkspaceService mock).
	getCurrentModel: () => ({ model: 'test-model', provider: 'openai' }),
	tryGetCurrentModel: () => undefined,
	getCombinedCustomPrompt: () => ''
}))

vi.mock('../lib', () => ({
	getModelContextWindow: () => 128000,
	// chatRequest builds the client config eagerly; the mocked runChatLoop never
	// uses these, so stub clients are enough.
	workspaceAIClients: {
		subscribe: () => () => undefined,
		getOpenaiClient: () => ({}),
		getAnthropicClient: () => ({})
	}
}))

vi.mock('./api/apiTools', () => ({
	loadApiTools: vi.fn()
}))

// Mock only runChatLoop; keep the real truncateToToolPairedPrefix (pure) that
// the manager uses to commit partial output. importOriginal is safe here because
// chatLoop's heavy transitive deps (monaco via ../lib) are mocked above/below.
vi.mock('./chatLoop', async (importOriginal) => ({
	...(await importOriginal<typeof import('./chatLoop')>()),
	runChatLoop: vi.fn()
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
		vi.clearAllMocks()
	})

	it('restores the message to the composer when the model returns no output (#2)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const setInstructions = vi.fn()
		manager.setAiChatInput({ setInstructions, focusInput: vi.fn() } as any)

		// Empty turn: the loop produces no messages and no display output.
		vi.mocked(runChatLoop).mockResolvedValue({
			addedMessages: [],
			tokenUsage: {} as any,
			hitMaxIterations: false
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		// The empty user turn is rolled back out of the transcript...
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(false)
		expect(manager.messages.some((m) => m.role === 'user')).toBe(false)
		// ...and its text is handed back to the composer.
		expect(setInstructions).toHaveBeenCalledWith('do a thing', [])
		expect(manager.loading).toBe(false)
	})

	it('does NOT restore on a normal turn that produced output', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const setInstructions = vi.fn()
		manager.setAiChatInput({ setInstructions, focusInput: vi.fn() } as any)

		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.callbacks.onNewToken('hello')
			config.callbacks.onMessageEnd()
			return { addedMessages: [], tokenUsage: {} as any, hitMaxIterations: false }
		})

		manager.instructions = 'do a thing'
		await manager.sendRequest()

		expect(setInstructions).not.toHaveBeenCalled()
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(true)
		expect(manager.displayMessages.some((m) => m.role === 'assistant')).toBe(true)
	})

	it('keeps the tool-paired prefix of a failed turn as context, dropping the dangling call (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ setInstructions: vi.fn(), focusInput: vi.fn() } as any)

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
		manager.setAiChatInput({ setInstructions: vi.fn(), focusInput: vi.fn() } as any)

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
		const setInstructions = vi.fn()
		manager.setAiChatInput({ setInstructions, focusInput: vi.fn() } as any)

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
		expect(setInstructions).toHaveBeenCalledWith('think hard', [])
		expect(manager.loading).toBe(false)
	})

	it('restores the message when cancelled before any output at all (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const setInstructions = vi.fn()
		manager.setAiChatInput({ setInstructions, focusInput: vi.fn() } as any)

		// Cancelled immediately, before the model emitted anything.
		vi.mocked(runChatLoop).mockImplementation(async (config) => {
			config.abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.instructions = 'never mind'
		await manager.sendRequest()

		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(false)
		expect(manager.messages.some((m) => m.role === 'user')).toBe(false)
		expect(setInstructions).toHaveBeenCalledWith('never mind', [])
		expect(manager.loading).toBe(false)
	})

	it('keeps text flushed before a tool call when cancelled during the tool call (#3)', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		const setInstructions = vi.fn()
		manager.setAiChatInput({ setInstructions, focusInput: vi.fn() } as any)

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
		expect(setInstructions).not.toHaveBeenCalled()
		expect(manager.loading).toBe(false)
	})

	it('does not duplicate an already-committed answer when cancelled right after a completed message', async () => {
		const manager = new AIChatManager()
		manager.changeMode(AIMode.ASK)
		manager.setAiChatInput({ setInstructions: vi.fn(), focusInput: vi.fn() } as any)

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
})
