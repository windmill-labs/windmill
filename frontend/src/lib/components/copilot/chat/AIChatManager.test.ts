import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'

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
	getCurrentModel: () => undefined,
	tryGetCurrentModel: () => undefined,
	getCombinedCustomPrompt: () => ''
}))

vi.mock('../lib', () => ({
	getModelContextWindow: () => 128000,
	workspaceAIClients: { subscribe: () => () => undefined }
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

describe('AIChatManager context trim gate', () => {
	// Mocked window is 128000; threshold = 128000 - max(128000*0.05, 5000) = 121600 tokens.
	// chars÷4, so ~486400 chars to exceed.
	const big = (chars: number) => 'x'.repeat(chars)

	beforeEach(() => {
		vi.clearAllMocks()
	})

	it('counts the system message in the estimate (closes the overflow blind spot)', () => {
		const manager = new AIChatManager()
		const messages = [{ role: 'user', content: 'hi' }] as any

		// Small messages alone are well under the limit...
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(false)

		// ...but a large system message must push it over (the old gate ignored it).
		manager.systemMessage = { role: 'system', content: big(520_000) }
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(true)
	})

	it('counts the tool schemas in the estimate', () => {
		const manager = new AIChatManager()
		const messages = [{ role: 'user', content: 'hi' }] as any

		expect(manager.checkTokenUsageOverLimit(messages)).toBe(false)

		manager.tools = [{ def: { name: 'huge', schema: big(520_000) } }] as any
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(true)
	})

	it('drops an assistant tool-call message together with its tool response', () => {
		const manager = new AIChatManager()
		const messages = [
			{ role: 'assistant', content: null, tool_calls: [{ id: 't1', function: { name: 'f' } }] },
			{ role: 'tool', tool_call_id: 't1', content: big(300_000) },
			{ role: 'user', content: big(300_000) }
		] as any

		// 3 messages (~600k chars = 150k tokens) exceed the 121.6k threshold; the lone
		// user message (~300k chars / 4 = 75k tokens) is under it.
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(true)

		const trimmed = manager.deleteOldestMessage([...messages])

		// The assistant+tool pair is removed together — never a dangling tool at the front.
		expect(trimmed.map((m: any) => m.role)).toEqual(['user'])
		expect(manager.checkTokenUsageOverLimit(trimmed)).toBe(false)
	})

	it('calibrates the estimate to the accurate anchor when one exists', () => {
		const manager = new AIChatManager()
		const messages = [{ role: 'user', content: big(200_000) }] as any // crude ≈ 50k tokens

		// Without an anchor the crude estimate (~50k) is under the 121.6k threshold.
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(false)

		// Restoring a chat whose real context was ~130k tokens sets the anchor; the gate
		// scales the crude estimate by anchor/crude and now reports over-limit.
		;(manager.historyManager as any).loadPastChat = () => ({
			displayMessages: [],
			actualMessages: messages,
			contextTokens: 130_000
		})
		manager.loadPastChat('id')
		expect(manager.checkTokenUsageOverLimit(messages)).toBe(true)
	})
})
