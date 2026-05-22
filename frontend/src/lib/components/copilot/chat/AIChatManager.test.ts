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
	workspaceStore: { subscribe: () => () => undefined }
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

function createFlowHelpers({
	hasPendingChanges,
	acceptAllModuleActions
}: {
	hasPendingChanges: () => boolean
	acceptAllModuleActions: () => void
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
		testFlow: vi.fn(),
		getLintErrors: vi.fn()
	} as unknown as FlowAIChatHelpers
}

describe('AIChatManager autonomy mode', () => {
	beforeEach(() => {
		localStorage.clear()
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
})
