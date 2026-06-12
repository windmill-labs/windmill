import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'

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

vi.mock('./chatLoop', () => ({
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

describe('AIChatManager context usage estimation', () => {
	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
	})

	it('estimates messages, system prompt and tool defs when no anchor exists', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400) },
			{ role: 'assistant', content: 'b'.repeat(200) }
		]
		manager.systemMessage = { role: 'system', content: 'c'.repeat(100) }
		const def = { type: 'function' as const, function: { name: 'x' } }
		manager.tools = [{ def }] as any
		expect(manager.estimatedContextTokens).toBe(175 + JSON.stringify([def]).length / 4)
	})

	it('anchors on provider-reported usage and only estimates messages added since', () => {
		const manager = new AIChatManager()
		manager.messages = [
			// covered by the anchor: must not be re-estimated
			{ role: 'user', content: 'a'.repeat(99999) },
			{ role: 'user', content: 'b'.repeat(400) }
		]
		// covered by the anchor too: must not be added on top
		manager.systemMessage = { role: 'system', content: 'c'.repeat(100) }
		manager.contextUsage = { tokens: 1000, atMessageIndex: 1 }
		expect(manager.estimatedContextTokens).toBe(1100)
	})

	it('falls back to full estimation when the anchor points past the current history', () => {
		const manager = new AIChatManager()
		manager.messages = [{ role: 'user', content: 'a'.repeat(400) }]
		manager.systemMessage = { role: 'system', content: '' }
		manager.contextUsage = { tokens: 5000, atMessageIndex: 3 }
		expect(manager.estimatedContextTokens).toBe(100)
	})

	it('clears the anchor when saveAndClear resets the conversation', async () => {
		const manager = new AIChatManager()
		manager.contextUsage = { tokens: 1000, atMessageIndex: 1 }
		await manager.saveAndClear()
		expect(manager.contextUsage).toBeUndefined()
	})
})
