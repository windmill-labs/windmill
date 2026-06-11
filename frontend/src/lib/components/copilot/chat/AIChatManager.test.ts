import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { runChatLoop } from './chatLoop'
import { emptyChatTokenUsage } from './tokenUsage'

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
	getCurrentModel: () => ({ model: 'test-model', provider: 'openai' }),
	tryGetCurrentModel: () => undefined,
	getCombinedCustomPrompt: () => ''
}))

vi.mock('../lib', () => ({
	getModelContextWindow: () => 128000,
	workspaceAIClients: {
		subscribe: () => () => undefined,
		getOpenaiClient: () => ({}),
		getAnthropicClient: () => ({})
	}
}))

vi.mock('./api/apiTools', () => ({
	loadApiTools: vi.fn()
}))

vi.mock('./chatLoop', () => ({
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

describe('AIChatManager queued messages', () => {
	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
	})

	function createInputMock() {
		return {
			prependText: vi.fn(),
			focusInput: vi.fn()
		}
	}

	function createManager(input?: ReturnType<typeof createInputMock>) {
		const manager = new AIChatManager()
		manager.mode = AIMode.NAVIGATOR
		if (input) {
			manager.setAiChatInput(input as unknown as Parameters<typeof manager.setAiChatInput>[0])
		}
		return manager
	}

	it('queues trimmed messages in order and ignores blank input', () => {
		const manager = createManager()
		manager.queueMessage('  first  ')
		manager.queueMessage('   ')
		manager.queueMessage('second')
		expect(manager.queuedMessages).toEqual(['first', 'second'])
	})

	it('dequeues one message by index and restores it into the input', () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.queuedMessages = ['a', 'b', 'c']

		manager.dequeueMessage(1)

		expect(manager.queuedMessages).toEqual(['a', 'c'])
		expect(input.prependText).toHaveBeenCalledWith('b')
	})

	it('re-queues instead of dropping when the input is unmounted', () => {
		const manager = createManager()
		manager.queuedMessages = ['a', 'b']

		manager.dequeueMessage(1)

		// no input to restore into → the message goes back to the queue front
		expect(manager.queuedMessages).toEqual(['b', 'a'])
	})

	it('auto-sends queued messages one per turn in FIFO order on success', async () => {
		const manager = createManager(createInputMock())
		vi.mocked(runChatLoop).mockResolvedValue({
			addedMessages: [],
			tokenUsage: emptyChatTokenUsage(),
			hitMaxIterations: false
		})

		manager.queuedMessages = ['second', 'third']
		await manager.sendRequest({ instructions: 'first' })

		expect(vi.mocked(runChatLoop)).toHaveBeenCalledTimes(3)
		expect(manager.queuedMessages).toEqual([])
		const userMessages = manager.displayMessages
			.filter((m) => m.role === 'user')
			.map((m) => m.content)
		expect(userMessages).toEqual(['first', 'second', 'third'])
	})

	it('restores all queued text to the input instead of sending when the turn errors', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		vi.mocked(runChatLoop).mockRejectedValue(new Error('provider down'))

		manager.queuedMessages = ['second', 'third']
		await manager.sendRequest({ instructions: 'first' })

		expect(vi.mocked(runChatLoop)).toHaveBeenCalledTimes(1)
		expect(manager.queuedMessages).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('second\n\nthird')
	})

	it('restores all queued text to the input when the turn is cancelled', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		vi.mocked(runChatLoop).mockImplementation(async ({ abortController }) => {
			abortController.abort()
			throw new Error('aborted')
		})

		manager.queuedMessages = ['second']
		await manager.sendRequest({ instructions: 'first' })

		expect(manager.queuedMessages).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('second')
	})

	it('restores a queued message whose auto-send is rejected by beforeSend', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		vi.mocked(runChatLoop).mockResolvedValue({
			addedMessages: [],
			tokenUsage: emptyChatTokenUsage(),
			hitMaxIterations: false
		})
		// first turn goes through, the queued auto-send is rejected
		manager.beforeSend = vi
			.fn()
			.mockResolvedValueOnce(undefined)
			.mockRejectedValueOnce(new Error('workspace commit failed'))

		manager.queuedMessages = ['second']
		await manager.sendRequest({ instructions: 'first' })

		expect(vi.mocked(runChatLoop)).toHaveBeenCalledTimes(1)
		expect(manager.queuedMessages).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('second')
	})
})
