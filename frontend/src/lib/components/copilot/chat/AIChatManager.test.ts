import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { FlowAIChatHelpers } from './flow/core'
import type { CurrentEditor } from '$lib/components/flows/types'
import type { ReviewChangesOpts } from './monaco-adapter'
import type { ChatCompletionMessageParam } from 'openai/resources/chat/completions.mjs'
import type { AttachedImage } from './imageUtils'
import { AIChatManager, AIMode, AIAutonomyMode } from './AIChatManager.svelte'
import { runChatLoop } from './chatLoop'

// This suite forces esm-env BROWSER=true (below). That makes @sveltejs/kit's
// client runtime (pulled transitively via $lib/navigation) evaluate browser-only
// globals at import time and throw "location is not defined" under the node test
// env. Stub the two $app modules $lib/navigation needs so kit's client runtime is
// never loaded. File-local: no other suite is affected.
vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}))
vi.mock('$app/paths', () => ({ base: '', assets: '' }))

const mocks = vi.hoisted(() => ({
	getCurrentModel: vi.fn(),
	tryGetCurrentModel: vi.fn(),
	isWebSearchEnabledForProvider: vi.fn(),
	logAiChat: vi.fn(),
	sendUserToast: vi.fn(),
	getOpenaiClient: vi.fn(),
	getAnthropicClient: vi.fn(),
	getNonStreamingCompletion: vi.fn(),
	runChatLoop: vi.fn(),
	listAiSkills: vi.fn(),
	getJob: vi.fn(),
	workspace: 'test_workspace' as string | undefined
}))

vi.mock('monaco-editor', () => ({
	Selection: class Selection {}
}))

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		logAiChat: mocks.logAiChat,
		listAiSkills: mocks.listAiSkills
	},
	ScriptService: {},
	FlowService: {},
	JobService: {
		getJob: mocks.getJob
	}
}))

// Autonomy mode is now namespaced by the logged-in user's email (see
// userScopedStorage); the mock emits one so scopedKey() resolves.
const TEST_EMAIL = 'admin@test'

vi.mock('$lib/stores', () => {
	// A minimal readable store: get(store) reads this value synchronously. Defined
	// inside the factory since vi.mock is hoisted above module-scope declarations.
	const readable = <T>(value: T) => ({
		subscribe: (run: (v: T) => void) => {
			run(value)
			return () => undefined
		}
	})
	return {
		workspaceStore: {
			subscribe: (run: (value: string | undefined) => void) => {
				run(mocks.workspace)
				return () => undefined
			}
		},
		userStore: readable({ username: 'admin', email: 'admin@test', is_admin: true }),
		// Read eagerly at module load by the open_page tool's allowedOpenPages /
		// allowedTriggerKinds (global/core.ts) as the manager's tools are built.
		superadmin: readable(false),
		userWorkspaces: readable([] as unknown[]),
		enterpriseLicense: readable(undefined)
	}
})

vi.mock('$lib/toast', () => ({
	sendUserToast: mocks.sendUserToast
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: mocks.getCurrentModel,
	tryGetCurrentModel: mocks.tryGetCurrentModel,
	getCombinedCustomPrompt: () => '',
	getCustomPromptParts: () => ({}),
	getUserCustomPrompts: () => ({}),
	setUserCustomPrompts: () => {},
	isWebSearchEnabledForProvider: mocks.isWebSearchEnabledForProvider
}))

vi.mock('../lib', () => ({
	workspaceAIClients: {
		subscribe: () => () => undefined,
		getOpenaiClient: mocks.getOpenaiClient,
		getAnthropicClient: mocks.getAnthropicClient
	},
	getNonStreamingCompletion: mocks.getNonStreamingCompletion
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
	mocks.listAiSkills.mockResolvedValue([])
	mocks.workspace = 'test_workspace'
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

describe('AIChatManager global skills', () => {
	const model = { provider: 'openai', model: 'gpt-4o' }

	beforeEach(() => {
		localStorage.clear()
		mocks.getCurrentModel.mockReturnValue(model)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	it('loads skills after beforeSend commits the session workspace', async () => {
		let resolveParentSkills: ((skills: { name: string; description: string }[]) => void) | undefined
		const parentSkills = new Promise<{ name: string; description: string }[]>((resolve) => {
			resolveParentSkills = resolve
		})
		mocks.workspace = 'parent'
		mocks.listAiSkills.mockImplementation(({ workspace }: { workspace: string }) => {
			if (workspace === 'parent') {
				return parentSkills
			}
			return Promise.resolve([{ name: 'child-skill', description: 'child workspace skill' }])
		})
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			expect(config.workspace).toBe('child')
			expect(config.systemMessage.content).toContain('child-skill')
			expect(config.systemMessage.content).not.toContain('parent-skill')
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})

		const manager = new AIChatManager()
		manager.isSessionChat = true
		manager.beforeSend = () => {
			mocks.workspace = 'child'
		}

		await manager.sendRequest({ instructions: 'first', mode: AIMode.GLOBAL })
		resolveParentSkills?.([{ name: 'parent-skill', description: 'parent workspace skill' }])
		await Promise.resolve()

		expect(mocks.listAiSkills).toHaveBeenCalledWith({ workspace: 'parent' })
		expect(mocks.listAiSkills).toHaveBeenCalledWith({ workspace: 'child' })
		expect(manager.systemMessage.content).toContain('child-skill')
		expect(manager.systemMessage.content).not.toContain('parent-skill')
	})

	it('expands a leading slash skill command for the model while preserving the displayed text', async () => {
		mocks.listAiSkills.mockResolvedValue([
			{ name: 'review-code', description: 'review code for bugs' }
		])
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const userMessage = config.messages[config.messages.length - 1]
			expect(userMessage.content).toContain('Use the "review-code" skill. find bugs')
			expect(userMessage.content).not.toContain('/review-code find bugs')
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})

		const manager = new AIChatManager()
		manager.isSessionChat = true

		await manager.sendRequest({ instructions: '/review-code find bugs', mode: AIMode.GLOBAL })

		expect(manager.displayMessages[0]?.content).toBe('/review-code find bugs')
	})
})

describe('AIChatManager autonomy mode', () => {
	beforeEach(() => {
		localStorage.clear()
		// These tests exercise the transition into auto-accept, so start from the
		// ask-permission baseline rather than the new auto-accept-edits default.
		localStorage.setItem(`ai-chat-autonomy-mode::${TEST_EMAIL}`, AIAutonomyMode.DEFAULT)
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
	// Mirrors the private storage keys in AIChatManager.svelte.ts, namespaced by
	// the logged-in user's email (see userScopedStorage).
	const AUTONOMY_KEY = `ai-chat-autonomy-mode::${TEST_EMAIL}`
	const LEGACY_YOLO_KEY = `ai-chat-yolo-mode::${TEST_EMAIL}`

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
	const model = { provider: 'openai', model: 'gpt-4o' }

	// The turn-outcome handling rolls back turns with no usable output, so a
	// "successful" send must produce a reply to take the clean-commit path
	// (which is what gates the queued-message auto-send).
	const replyWith = (reply: string) =>
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: reply }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})

	beforeEach(() => {
		localStorage.clear()
		mocks.getCurrentModel.mockReturnValue(model)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	function createInputMock() {
		return {
			prependText: vi.fn(),
			restoreInstructions: vi.fn(),
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

	it('queues a single trimmed message and ignores blank input', () => {
		const manager = createManager()
		manager.queueMessage('  first  ')
		manager.queueMessage('   ')
		expect(manager.queuedMessage).toBe('first')
	})

	it('appends additional lines to the single queued message', () => {
		const manager = createManager()
		manager.queueMessage('first line')
		manager.queueMessage('second line')
		expect(manager.queuedMessage).toBe('first line\nsecond line')
	})

	it('dequeues the message and restores it into the input', () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.queuedMessage = 'line one\nline two'

		manager.dequeueMessage()

		expect(manager.queuedMessage).toBe('')
		expect(input.prependText).toHaveBeenCalledWith('line one\nline two', [])
	})

	const img = (n: string): AttachedImage => ({
		dataUrl: `data:image/png;base64,${n}`,
		mediaType: 'image/png',
		name: n
	})

	it('carries queued images through to the auto-send', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL // sendRequest only assembles images in GLOBAL
		manager.queueMessage('look at this', [img('a')])

		await manager.sendRequest({ instructions: 'first' })

		// The auto-sent turn must be the message the user actually submitted —
		// before, the text went out alone and the images were dropped on queueing.
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(2)
		const autoSent = manager.displayMessages.find(
			(m) => m.role === 'user' && m.content === 'look at this'
		)
		expect(autoSent && 'images' in autoSent ? autoSent.images : undefined).toEqual([img('a')])
		expect(manager.queuedImages).toEqual([])
	})

	// Attaching is refused on a text-only model, but the model can be switched
	// after attaching (or a screenshot buffered), and sending the image then fails
	// the whole turn. The send path re-checks rather than trusting the attach gate.
	it('drops images when the model in use cannot read them', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		// a real bundled default, so this exercises the actual gate rather than a mock
		mocks.tryGetCurrentModel.mockReturnValue({ provider: 'groq', model: 'llama-3.3-70b-versatile' })

		await manager.sendRequest({ instructions: 'look', images: [img('a')] })

		const bubble = manager.displayMessages.find((m) => m.role === 'user')
		expect(bubble && 'images' in bubble ? bubble.images : undefined).toBeUndefined()
		expect(mocks.sendUserToast).toHaveBeenCalledWith(
			expect.stringContaining("can't read images"),
			true
		)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// displayMessages hold a 384px transcript copy; retrying must resend the
	// model's own 1568px image, not a thumbnail of its previous input.
	it('resends the model-resolution image on retry, not the transcript thumbnail', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.messages = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'look' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,FULLRES' } }
				] as any
			},
			{ role: 'assistant', content: 'bad answer' }
		]
		manager.displayMessages = [
			{
				role: 'user',
				content: 'look',
				index: 0,
				images: [{ dataUrl: 'data:image/png;base64,THUMB', mediaType: 'image/png' }]
			},
			{ role: 'assistant', content: 'bad answer' }
		]

		manager.restartGeneration(0)
		await vi.waitFor(() => expect(mocks.runChatLoop).toHaveBeenCalled())

		const resent = mocks.runChatLoop.mock.calls[0][0].messages.at(-1)
		const urls = (resent.content as any[])
			.filter((p) => p.type === 'image_url')
			.map((p) => p.image_url.url)
		expect(urls).toEqual(['data:image/png;base64,FULLRES'])
	})

	// The gate on this turn's images is not enough: history keeps image parts from
	// earlier turns, and a text-only model rejects the whole request over them.
	it('strips historical images from the outbound copy on a text-only model', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.messages = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'earlier turn' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,OLD' } }
				] as any
			},
			{ role: 'assistant', content: 'ok' }
		]
		mocks.tryGetCurrentModel.mockReturnValue({ provider: 'groq', model: 'llama-3.3-70b-versatile' })

		await manager.sendRequest({ instructions: 'plain text follow-up' })

		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		const anyImage = sent.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(anyImage).toBe(false)
		// stored history keeps them, so switching back to a vision model still works
		expect((manager.messages[0].content as any[]).some((p) => p.type === 'image_url')).toBe(true)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// Queuing clears the composer, so its own counter resets; the cap has to hold
	// on the queue or repeated sends stack an unbounded batch into one message.
	it('caps images accumulated across repeated queued sends', () => {
		const manager = createManager()
		for (let i = 0; i < 4; i++) {
			manager.queueMessage(`msg ${i}`, [img(`a${i}`), img(`b${i}`), img(`c${i}`)])
		}
		expect(manager.queuedImages.length).toBe(8)
	})

	// A rejected image stays in history, so every later turn resends it and fails
	// the same way — the conversation wedges with no way out but editing or /clear.
	it('removes the image from history when the provider rejects it', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.runChatLoop.mockImplementation(async () => {
			throw new Error('400 Invalid image content')
		})

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(false)
		// the prompt itself survives, so a follow-up still has the text as context
		expect(manager.messages.length).toBeGreaterThan(0)
		expect(mocks.sendUserToast).toHaveBeenCalledWith(
			expect.stringContaining('could not read the attached image'),
			true
		)
	})

	// An unrelated failure must not strip a perfectly good image.
	it('keeps the image when the failure is unrelated', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.runChatLoop.mockImplementation(async () => {
			throw new Error('429 rate limit exceeded')
		})

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(true)
	})

	// The rejection fallback strips the image from history but leaves the bubble's
	// thumbnail. Retry must not resurrect it, or the retried turn fails identically
	// and the conversation is wedged after all.
	it('does not resend an image the fallback already stripped', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		// post-rejection shape: history stripped to text, transcript still shows it
		manager.messages = [{ role: 'user', content: 'look at this\n[image omitted]' }]
		manager.displayMessages = [
			{ role: 'user', content: 'look at this', index: 0, images: [img('thumb')] }
		]

		manager.restartGeneration(0)
		await vi.waitFor(() => expect(mocks.runChatLoop).toHaveBeenCalled())

		const resent = mocks.runChatLoop.mock.calls[0][0].messages.at(-1)
		const hasImage =
			Array.isArray(resent.content) && resent.content.some((p: any) => p.type === 'image_url')
		expect(hasImage).toBe(false)
	})

	it('restores queued images to the input on dequeue', () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.queueMessage('with a picture', [img('a')])

		manager.dequeueMessage()

		expect(manager.queuedImages).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('with a picture', [img('a')])
	})

	it('drops queued images when the conversation is switched away', async () => {
		const manager = createManager(createInputMock())
		manager.queueMessage('stale', [img('a')])

		await manager.saveAndClear()

		// images must not survive into the next conversation
		expect(manager.queuedMessage).toBe('')
		expect(manager.queuedImages).toEqual([])
	})

	it('re-queues instead of dropping when the input is unmounted', () => {
		const manager = createManager()
		manager.queuedMessage = 'keep me'

		manager.dequeueMessage()

		// no input to restore into → the message stays queued
		expect(manager.queuedMessage).toBe('keep me')
	})

	it('auto-sends the queued message on a clean completion', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'first' })

		expect(mocks.runChatLoop).toHaveBeenCalledTimes(2)
		expect(manager.queuedMessage).toBe('')
		const userMessages = manager.displayMessages
			.filter((m) => m.role === 'user')
			.map((m) => m.content)
		expect(userMessages).toEqual(['first', 'followup'])
	})

	it('keeps the queued message as a card (not flushed to input) when the turn errors', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		mocks.runChatLoop.mockRejectedValue(new Error('provider down'))

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'first' })

		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
		// stays a card, nothing flushed into the input
		expect(manager.queuedMessage).toBe('followup')
		expect(input.prependText).not.toHaveBeenCalled()
	})

	it('auto-sends the queued message when the user cancels the turn (Esc/Stop)', async () => {
		const manager = createManager(createInputMock())
		// the followup turn completes cleanly...
		replyWith('done')
		// ...but the first turn is cancelled by the user
		mocks.runChatLoop.mockImplementationOnce(async ({ abortController }: any) => {
			abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'first' })

		// cancel sends the queued message automatically
		expect(manager.queuedMessage).toBe('')
		const userMessages = manager.displayMessages
			.filter((m) => m.role === 'user')
			.map((m) => m.content)
		expect(userMessages).toContain('followup')
	})

	it('does NOT auto-send on a programmatic cancel (e.g. save-and-clear / teardown)', async () => {
		const manager = createManager(createInputMock())
		replyWith('done')
		// the turn is aborted programmatically, not by the user pressing Esc/Stop
		mocks.runChatLoop.mockImplementationOnce(async ({ abortController }: any) => {
			abortController.abort('saveAndClear')
			throw new Error('aborted')
		})

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'first' })

		// a non-user abort must not fire the queued message; it stays a card
		expect(manager.queuedMessage).toBe('followup')
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
	})

	it('does not restore the cancelled prompt to the input when a queued message takes over', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		replyWith('done')
		// cancel before any usable output → the rollback (restoreUnsentTurn) path
		mocks.runChatLoop.mockImplementationOnce(async ({ abortController }: any) => {
			abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'the long cancelled prompt' })

		// clean handoff: queued message sent, cancelled prompt NOT shoved back in
		expect(manager.queuedMessage).toBe('')
		expect(input.restoreInstructions).not.toHaveBeenCalled()
	})

	it('re-queues the message when its auto-send is rejected by beforeSend', async () => {
		replyWith('done')
		const input = createInputMock()
		const manager = createManager(input)
		// first turn goes through, the queued auto-send is rejected
		manager.beforeSend = vi
			.fn()
			.mockResolvedValueOnce(undefined)
			.mockRejectedValueOnce(new Error('workspace commit failed'))

		manager.queuedMessage = 'followup'
		await manager.sendRequest({ instructions: 'first' })

		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
		// the rejected message stays a card rather than being lost or moved to input
		expect(manager.queuedMessage).toBe('followup')
		expect(input.prependText).not.toHaveBeenCalled()
	})

	it('drops the queued message when switching conversations (no cross-chat leak)', async () => {
		const manager = createManager(createInputMock())

		manager.queuedMessage = 'meant for chat A'
		await manager.saveAndClear()
		expect(manager.queuedMessage).toBe('')

		manager.queuedMessage = 'still meant for chat A'
		vi.spyOn(manager.historyManager, 'loadPastChat').mockReturnValue({
			id: 'chat-b',
			title: 'Chat B',
			displayMessages: [],
			actualMessages: [],
			lastModified: 0
		} as unknown as ReturnType<typeof manager.historyManager.loadPastChat>)
		await manager.loadPastChat('chat-b')
		expect(manager.queuedMessage).toBe('')
	})

	it('clears attachments on New chat / load past chat (non-session), keeps them in a session', async () => {
		const txt = (n: string) => new File(['hello\n'], n, { type: 'text/plain' })

		// Non-session global chat: New chat must clear the previous conversation's attachments.
		const manager = createManager(createInputMock())
		await manager.attachedFiles.addFiles([txt('a.txt')])
		expect(manager.attachedFiles.count).toBe(1)
		await manager.saveAndClear()
		expect(manager.attachedFiles.count).toBe(0)

		// ...and loading a past chat clears them too.
		vi.spyOn(manager.historyManager, 'loadPastChat').mockReturnValue({
			id: 'chat-c',
			title: 'Chat C',
			displayMessages: [],
			actualMessages: [],
			lastModified: 0
		} as unknown as ReturnType<typeof manager.historyManager.loadPastChat>)
		await manager.attachedFiles.addFiles([txt('c.txt')])
		expect(manager.attachedFiles.count).toBe(1)
		await manager.loadPastChat('chat-c')
		expect(manager.attachedFiles.count).toBe(0)

		// Session chat: attachments are session-scoped — they survive New chat.
		const session = createManager(createInputMock())
		session.isSessionChat = true
		await session.attachedFiles.addFiles([txt('b.txt')])
		await session.saveAndClear()
		expect(session.attachedFiles.count).toBe(1)
	})

	it('tracks (empty mask) a session chat loaded with no stored modified-items', async () => {
		// A legacy session chat has no persisted mask. It must NOT stay untracked
		// (undefined) — that makes the Edits surface fall back to showing every
		// draft in the (possibly forked) workspace. Seed an empty tracked set so the
		// session only ever surfaces what it actually edited.
		const manager = createManager(createInputMock())
		manager.isSessionChat = true
		vi.spyOn(manager.historyManager, 'loadPastChat').mockReturnValue({
			id: 'legacy-session-chat',
			title: 'Legacy',
			displayMessages: [],
			actualMessages: [],
			lastModified: 0
		} as unknown as ReturnType<typeof manager.historyManager.loadPastChat>)
		vi.spyOn(manager.historyManager, 'getModifiedItems').mockReturnValue(undefined)

		await manager.loadPastChat('legacy-session-chat')

		expect(manager.modifiedItems).toBeInstanceOf(Set)
		expect(manager.modifiedItems?.size).toBe(0)
	})

	it('seeds a session chat mask from its stored modified-items', async () => {
		const manager = createManager(createInputMock())
		manager.isSessionChat = true
		vi.spyOn(manager.historyManager, 'loadPastChat').mockReturnValue({
			id: 'tracked-session-chat',
			title: 'Tracked',
			displayMessages: [],
			actualMessages: [],
			lastModified: 0
		} as unknown as ReturnType<typeof manager.historyManager.loadPastChat>)
		vi.spyOn(manager.historyManager, 'getModifiedItems').mockReturnValue([
			'script:u/admin/hello_world'
		])

		await manager.loadPastChat('tracked-session-chat')

		expect([...(manager.modifiedItems ?? [])]).toEqual(['script:u/admin/hello_world'])
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
		const saveChat = vi.spyOn(manager.historyManager, 'saveChat')

		await manager.sendRequest()

		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		expect(sent.length).toBe(3)
		expect(sent[0]).toMatchObject({ role: 'user', content: 'c'.repeat(400) })
		// The mutation is on the stored history, not a per-send copy: the head
		// pair is gone for good and the turn's reply was committed on top
		expect(manager.messages.length).toBe(4)
		expect(manager.messages[0]).toMatchObject({ role: 'user', content: 'c'.repeat(400) })
		// Mid-turn, the report is debited by the freed estimate (visible in the
		// compaction-time save) so a rolled-back turn keeps a consistent value
		// 4th arg: the modified-items mask rides on every save (undefined here —
		// this bare manager never initialised tracking).
		expect(saveChat).toHaveBeenCalledWith(expect.anything(), expect.anything(), 650_000, undefined)
		// At commit, the no-report turn clears the stored value; the readable
		// number falls back to estimating the now-tiny compacted history
		expect(manager.contextUsage).toBeUndefined()
		expect(manager.contextTokens).toBeGreaterThan(0)
		expect(manager.contextTokens).toBeLessThan(50_000)
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

	it('does not compact while the estimated context stays under the trigger', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) }
		]
		// no report: the trigger runs off the ~200k estimate, well under 800k
		manager.instructions = 'next question'

		await manager.sendRequest()

		expect(mocks.runChatLoop.mock.calls[0][0].messages.length).toBe(3)
	})

	it('compacts off the estimate alone when no report ever arrived', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(1_600_000) }, // ~400k estimated tokens
			{ role: 'assistant', content: 'b'.repeat(1_600_000) }, // ~400k
			{ role: 'user', content: 'c'.repeat(400) },
			{ role: 'assistant', content: 'd'.repeat(400) }
		]
		// ~800k estimated with no provider report ever seen (e.g. a gateway that
		// strips usage): the lazily-estimated projection trips the 800k trigger
		// and frees down to ~700k — the first user + assistant pair goes
		manager.instructions = 'next question'

		await manager.sendRequest()

		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		expect(sent.length).toBe(3)
		expect(sent[0]).toMatchObject({ role: 'user', content: 'c'.repeat(400) })
	})

	it('estimates lazily instead of storing a guess when the provider reports no usage', async () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) }
		]
		manager.instructions = 'next question'

		await manager.sendRequest() // replyWith('done') reports no usage

		// the stored value stays a pure provider fact…
		expect(manager.contextUsage).toBeUndefined()
		// …while the readable number estimates the stored context: ~200k for the
		// messages plus the real navigator system prompt, tool defs and the small
		// new-turn messages; the prompt templates aren't pinned here, so assert
		// the magnitude rather than the byte count
		expect(manager.contextTokens).toBeGreaterThan(200_000)
		expect(manager.contextTokens).toBeLessThan(250_000)
	})

	it('prefers the provider report over the estimate once one arrives', async () => {
		const manager = new AIChatManager()
		manager.messages = [{ role: 'user', content: 'a'.repeat(400) }]
		manager.instructions = 'first'
		await manager.sendRequest()
		expect(manager.contextUsage).toBeUndefined()
		expect(manager.contextTokens).toBeGreaterThan(0)

		replyWith('done', { prompt: 1_234, completion: 56, total: 1_290 })
		manager.instructions = 'second'
		await manager.sendRequest()
		expect(manager.contextUsage).toBe(1_290)
		expect(manager.contextTokens).toBe(1_290)
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

	it('falls back to estimating the rewound history after a rewind', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400) }, // ~100 estimated tokens
			{ role: 'assistant', content: 'b'.repeat(400) }, // ~100
			{ role: 'user', content: 'q2' },
			{ role: 'assistant', content: 'a2' }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'q1', index: 0 },
			{ role: 'assistant', content: 'a1' },
			{ role: 'user', content: 'q2', index: 2 },
			{ role: 'assistant', content: 'a2' }
		]
		// A report that described the pre-rewind history must not survive the
		// rewind as-is…
		manager.contextUsage = 999_999
		manager.restartGeneration(2)
		expect(manager.contextUsage).toBeUndefined()
		// …but the readable number stays armed by estimating what remains (the
		// two surviving messages, plus the prompt/tools the resend installed),
		// so e.g. Retry after a context-length error still compacts
		expect(manager.contextTokens).toBeGreaterThanOrEqual(200)
		expect(manager.contextTokens).toBeLessThan(50_000)
	})

	it('clears the reported usage when saveAndClear resets the conversation', async () => {
		const manager = new AIChatManager()
		manager.contextUsage = 1000
		await manager.saveAndClear()
		expect(manager.contextUsage).toBeUndefined()
	})

	// gpt-4o resolves to a known 128k window (modelConfig unmocked): trigger at
	// ~102k, target ~90k. With a summary reserve of 8k the tail budget is ~76k.
	const gpt4oModel = { provider: 'openai', model: 'gpt-4o' }

	// Older prefix (4 messages, ~25k tokens each = 100k chars) plus a recent
	// user+assistant pair that fits the tail budget. After the new user turn is
	// pushed the budget keeps [recentQ, recentA, new] verbatim and summarizes the
	// four old messages.
	function seedForSummary(manager: AIChatManager) {
		manager.messages = [
			{ role: 'user', content: 'OLD1' + 'a'.repeat(100_000) },
			{ role: 'assistant', content: 'OLD2' + 'b'.repeat(100_000) },
			{ role: 'user', content: 'OLD3' + 'c'.repeat(100_000) },
			{ role: 'assistant', content: 'OLD4' + 'd'.repeat(100_000) },
			{ role: 'user', content: 'recentQ' + 'e'.repeat(80_000) },
			{ role: 'assistant', content: 'recentA' + 'f'.repeat(80_000) }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'old1', index: 0 },
			{ role: 'assistant', content: 'old2' },
			{ role: 'user', content: 'old3', index: 2 },
			{ role: 'assistant', content: 'old4' },
			{ role: 'user', content: 'recentQ', index: 4 },
			{ role: 'assistant', content: 'recentA' }
		]
		manager.instructions = 'next question'
	}

	it('summarizes the older prefix and keeps the recent tail verbatim', async () => {
		mocks.getCurrentModel.mockReturnValue(gpt4oModel)
		mocks.tryGetCurrentModel.mockReturnValue(gpt4oModel)
		mocks.getNonStreamingCompletion.mockResolvedValue(
			'<analysis>scratchpad</analysis><summary>SUMMARY TEXT</summary>'
		)
		const manager = new AIChatManager()
		seedForSummary(manager)

		await manager.sendRequest()

		// The prefix (the four OLD messages) was sent to the summarizer, followed
		// by the summary-instruction user message.
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		const summaryReq = mocks.getNonStreamingCompletion.mock.calls[0][0]
		expect(summaryReq).toHaveLength(5)
		expect(summaryReq[0].content).toContain('OLD1')
		expect(summaryReq[3].content).toContain('OLD4')
		expect(summaryReq[4].content).toContain('detailed summary')

		// The request that went out begins with the summary user message, then the
		// recent tail verbatim, then the new question.
		const sent = mocks.runChatLoop.mock.calls[mocks.runChatLoop.mock.calls.length - 1][0].messages
		expect(sent).toHaveLength(4)
		expect(sent[0].role).toBe('user')
		expect(sent[0].content).toContain('SUMMARY TEXT')
		expect(sent[0].content).toContain('continued from a previous conversation')
		expect(sent[0].content).not.toContain('scratchpad')
		expect(sent[1].content).toContain('recentQ')

		// The display transcript replaces the summarized bubbles with one boundary
		// and re-bases the surviving tail's restart indices onto the new history.
		expect(manager.displayMessages[0]).toMatchObject({ role: 'summary', content: 'SUMMARY TEXT' })
		const recentQDisplay = manager.displayMessages.find(
			(m) => m.role === 'user' && m.content === 'recentQ'
		)
		expect(recentQDisplay && 'index' in recentQDisplay ? recentQDisplay.index : undefined).toBe(1)
		// No report describes the new history, so the readable number re-estimates
		// the now-small compacted context.
		expect(manager.contextUsage).toBeUndefined()
	})

	// A take_screenshot follow-up is a `user` message with no display counterpart
	// (appendPendingToolImages injects it). It must never become the tail
	// boundary: `messages` and `displayMessages` would then be sliced at
	// different turns and the cards in between would vanish from the transcript
	// while the model still sees them.
	it('never lands the tail boundary on a screenshot follow-up that has no display counterpart', async () => {
		mocks.getCurrentModel.mockReturnValue(gpt4oModel)
		mocks.tryGetCurrentModel.mockReturnValue(gpt4oModel)
		mocks.getNonStreamingCompletion.mockResolvedValue('<summary>SUMMARY TEXT</summary>')
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'OLD1' + 'a'.repeat(100_000) },
			{ role: 'assistant', content: 'OLD2' + 'b'.repeat(100_000) },
			{ role: 'user', content: 'OLD3' + 'c'.repeat(100_000) },
			{ role: 'assistant', content: 'toolTurn', tool_calls: [] as any },
			{ role: 'tool', content: 'Screenshot captured', tool_call_id: 't1' } as any,
			// the synthetic follow-up: user role, image parts, NO display entry
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'Screenshot(s) of the app preview:' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,AAAA' } }
				] as any
			},
			// sized so the tail budget breaks just above OLD3: the backward walk
			// stops at index 3, and the forward snap then lands on the synthetic
			// user message at 5 — the case this test exists for.
			{ role: 'assistant', content: 'afterShot' + 'g'.repeat(120_000) },
			{ role: 'user', content: 'recentQ' + 'h'.repeat(120_000) }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'old1', index: 0 },
			{ role: 'assistant', content: 'old2' },
			{ role: 'user', content: 'old3', index: 2 },
			{ role: 'assistant', content: 'afterShot' },
			{ role: 'user', content: 'recentQ', index: 7 }
		]
		manager.contextUsage = 110_000 // over the 0.8 * 128k trigger
		manager.instructions = 'next question'

		await manager.sendRequest()

		// Whatever survived summarization, the two views must agree: any assistant
		// turn the model can still see must still be visible to the user.
		const keptAfterShot = manager.messages.some(
			(m) => typeof m.content === 'string' && m.content.includes('afterShot')
		)
		const shownAfterShot = manager.displayMessages.some(
			(m) => m.role === 'assistant' && m.content.includes('afterShot')
		)
		expect(shownAfterShot).toBe(keptAfterShot)
	})

	it('falls back to drop-oldest when summarization fails', async () => {
		mocks.getCurrentModel.mockReturnValue(gpt4oModel)
		mocks.tryGetCurrentModel.mockReturnValue(gpt4oModel)
		mocks.getNonStreamingCompletion.mockRejectedValue(new Error('summary boom'))
		const manager = new AIChatManager()
		seedForSummary(manager)

		await manager.sendRequest()

		// Summarization was attempted, then the request still went out — via
		// drop-oldest, so no summary boundary anywhere.
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		expect(sent[0].content).not.toContain('continued from a previous conversation')
		expect(manager.displayMessages.some((m) => m.role === 'summary')).toBe(false)
	})

	it('skips summarization (drop-oldest) when the prefix is too small', async () => {
		mocks.getCurrentModel.mockReturnValue(anthropicModel)
		mocks.tryGetCurrentModel.mockReturnValue(anthropicModel)
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'a'.repeat(400_000) },
			{ role: 'assistant', content: 'b'.repeat(400_000) },
			{ role: 'user', content: 'c'.repeat(400) }
		]
		manager.contextUsage = 850_000
		manager.instructions = 'next question'

		await manager.sendRequest()

		// A two-message prefix isn't worth a summary round-trip.
		expect(mocks.getNonStreamingCompletion).not.toHaveBeenCalled()
		expect(manager.displayMessages.some((m) => m.role === 'summary')).toBe(false)
	})

	it('does not drop-oldest compact when the user stops during summarization', async () => {
		mocks.getCurrentModel.mockReturnValue(gpt4oModel)
		mocks.tryGetCurrentModel.mockReturnValue(gpt4oModel)
		// The user hits Stop while the summary request is in flight: it aborts the
		// turn's controller and rejects.
		mocks.getNonStreamingCompletion.mockImplementation(async (_msgs: any, ac: AbortController) => {
			ac.abort('user_cancelled')
			throw new Error('aborted')
		})
		// With the controller already aborted, the real request returns nothing;
		// mirror that so the turn takes the cancel/rollback path.
		mocks.runChatLoop.mockImplementation(async () => ({
			addedMessages: [],
			tokenUsage: { prompt: 0, completion: 0, total: 0 },
			lastIterationUsage: null,
			hitMaxIterations: false
		}))
		const manager = new AIChatManager()
		seedForSummary(manager)

		await manager.sendRequest()

		// Summarization was attempted and aborted, but the abort must NOT trigger a
		// destructive drop-oldest fallback: the full prefix survives and the unsent
		// turn is rolled back to the pre-send history (the head pair is still there).
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		expect(manager.messages).toHaveLength(6)
		expect(manager.messages[0].content).toContain('OLD1')
		expect(manager.displayMessages.some((m) => m.role === 'summary')).toBe(false)
	})
})

describe('AIChatManager manual compaction', () => {
	const model = { provider: 'openai', model: 'gpt-4o' }

	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
		mocks.getCurrentModel.mockReturnValue(model)
		mocks.tryGetCurrentModel.mockReturnValue(model)
		// changeMode(GLOBAL) refreshes workspace skills; keep it a no-op here.
		mocks.listAiSkills.mockResolvedValue([])
	})

	function seedExchange(manager: AIChatManager) {
		manager.messages = [
			{ role: 'user', content: 'q1' },
			{ role: 'assistant', content: 'a1' },
			{ role: 'user', content: 'q2' },
			{ role: 'assistant', content: 'a2' }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'q1', index: 0 },
			{ role: 'assistant', content: 'a1' },
			{ role: 'user', content: 'q2', index: 2 },
			{ role: 'assistant', content: 'a2' }
		]
	}

	it('folds the whole history into a single summary boundary, keeping nothing verbatim', async () => {
		mocks.getNonStreamingCompletion.mockResolvedValue('<summary>MANUAL SUMMARY</summary>')
		const manager = new AIChatManager()
		seedExchange(manager)
		manager.contextUsage = 123
		const saveChat = vi.spyOn(manager.historyManager, 'saveChat')

		await manager.compactManually()

		// The summarizer saw the entire history, then the summary instruction.
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		const summaryReq = mocks.getNonStreamingCompletion.mock.calls[0][0]
		expect(summaryReq).toHaveLength(5)
		expect(summaryReq[0].content).toBe('q1')
		expect(summaryReq[3].content).toBe('a2')
		expect(summaryReq[4].content).toContain('detailed summary')

		// Nothing kept verbatim: messages collapse to just the summary user message.
		expect(manager.messages).toHaveLength(1)
		expect(manager.messages[0].role).toBe('user')
		expect(manager.messages[0].content).toContain('MANUAL SUMMARY')
		expect(manager.messages[0].content).toContain('continued from a previous conversation')
		expect(manager.messages[0].content).not.toContain('<summary>')

		// The transcript shows one summary boundary in place of the old bubbles.
		expect(manager.displayMessages).toHaveLength(1)
		expect(manager.displayMessages[0]).toMatchObject({ role: 'summary', content: 'MANUAL SUMMARY' })

		expect(manager.contextUsage).toBeUndefined()
		expect(saveChat).toHaveBeenCalled()
		expect(mocks.sendUserToast).toHaveBeenCalledWith('Conversation compacted.')
		expect(manager.loading).toBe(false)
		expect(manager.compacting).toBe(false)
	})

	it('no-ops with a toast when there is nothing worth compacting', async () => {
		const manager = new AIChatManager()
		manager.messages = [{ role: 'user', content: 'only one' }]

		await manager.compactManually()

		expect(mocks.getNonStreamingCompletion).not.toHaveBeenCalled()
		expect(mocks.sendUserToast).toHaveBeenCalledWith('Nothing to compact yet.')
		expect(manager.messages).toHaveLength(1)
	})

	it('leaves history untouched when the user stops mid-summary', async () => {
		mocks.getNonStreamingCompletion.mockImplementation(async (_msgs: any, ac: AbortController) => {
			ac.abort('user_cancelled')
			throw new Error('aborted')
		})
		const manager = new AIChatManager()
		seedExchange(manager)

		await manager.compactManually()

		expect(manager.messages).toHaveLength(4)
		expect(manager.displayMessages.some((m) => m.role === 'summary')).toBe(false)
		// An abort is a user cancel, not a failure — no toast, no destructive change.
		expect(mocks.sendUserToast).not.toHaveBeenCalled()
		expect(manager.loading).toBe(false)
	})

	it('routes the /compact session command to manual compaction instead of the model', async () => {
		mocks.getNonStreamingCompletion.mockResolvedValue('<summary>VIA COMMAND</summary>')
		const manager = new AIChatManager()
		manager.isSessionChat = true
		seedExchange(manager)

		const sent = await manager.sendRequest({ instructions: '/compact', mode: AIMode.GLOBAL })

		// Consumed as a local command (true so the queue flush won't re-fire it),
		// without ever reaching the model loop...
		expect(sent).toBe(true)
		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		// ...it ran the summarizer and compacted in place, clearing the composer.
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		expect(manager.displayMessages[0]).toMatchObject({ role: 'summary', content: 'VIA COMMAND' })
		expect(manager.instructions).toBe('')
	})

	it('auto-sends a message queued while compaction was running', async () => {
		mocks.getNonStreamingCompletion.mockResolvedValue('<summary>S</summary>')
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})
		const manager = new AIChatManager()
		manager.isSessionChat = true
		manager.changeMode(AIMode.GLOBAL)
		seedExchange(manager)
		// A message typed while loading was true gets queued, not sent.
		manager.queuedMessage = 'follow-up question'

		await manager.compactManually()

		// Compaction ran once, then the queued message went out as a real turn.
		expect(mocks.getNonStreamingCompletion).toHaveBeenCalledTimes(1)
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
		const sent = mocks.runChatLoop.mock.calls[0][0].messages
		expect(sent[sent.length - 1].content).toContain('follow-up question')
		expect(manager.queuedMessage).toBe('')
	})

	it('routes the /clear session command to a fresh chat instead of the model', async () => {
		const manager = new AIChatManager()
		manager.isSessionChat = true
		seedExchange(manager)

		const sent = await manager.sendRequest({ instructions: '/clear', mode: AIMode.GLOBAL })

		// Consumed as a local command (true so the queue flush won't re-fire it),
		// without ever reaching the model...
		expect(sent).toBe(true)
		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		expect(mocks.getNonStreamingCompletion).not.toHaveBeenCalled()
		// ...it reset the conversation and cleared the composer.
		expect(manager.displayMessages).toEqual([])
		expect(manager.messages).toEqual([])
		expect(manager.instructions).toBe('')
	})

	it('consumes a /clear flushed from the queue without re-queuing it', async () => {
		// A normal turn that commits cleanly, so its epilogue flushes the queue.
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})
		const manager = new AIChatManager()
		manager.isSessionChat = true
		manager.changeMode(AIMode.GLOBAL)
		seedExchange(manager)
		// `/clear` typed while the turn was streaming gets queued, not sent.
		manager.queuedMessage = '/clear'

		await manager.sendRequest({ instructions: 'a normal message', mode: AIMode.GLOBAL })

		// The committed turn's flush ran `/clear` (resetting the conversation) and,
		// because the command reports itself as consumed, did NOT restore it — so a
		// stale `/clear` can't re-fire and wipe the next conversation.
		expect(manager.queuedMessage).toBe('')
		expect(manager.displayMessages).toEqual([])
		expect(manager.messages).toEqual([])
	})

	it('does not intercept /clear outside session chat', async () => {
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})
		const manager = new AIChatManager()
		manager.isSessionChat = false

		await manager.sendRequest({ instructions: '/clear', mode: AIMode.GLOBAL })

		// Without the session-chat command surface, /clear is a normal message.
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
	})

	it('does not intercept /compact outside session chat', async () => {
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			const message = { role: 'assistant' as const, content: 'done' }
			config.addedMessages?.push(message)
			return {
				addedMessages: [message],
				tokenUsage: { prompt: 0, completion: 0, total: 0 },
				hitMaxIterations: false
			}
		})
		const manager = new AIChatManager()
		manager.isSessionChat = false

		await manager.sendRequest({ instructions: '/compact', mode: AIMode.GLOBAL })

		// Without the session-chat command surface, /compact is a normal message.
		expect(mocks.runChatLoop).toHaveBeenCalledTimes(1)
		expect(mocks.getNonStreamingCompletion).not.toHaveBeenCalled()
	})

	it('shadows a workspace skill that collides with a built-in command', () => {
		const manager = new AIChatManager()
		manager.globalSkills = [
			{ name: 'compact', description: 'a workspace skill that happens to be named compact' },
			{ name: 'review-code', description: 'review code for bugs' }
		]

		// Built-ins come first and the colliding skill is dropped, so the picker
		// never renders two leaves with the same `skill:compact` key.
		const names = manager.sessionCommands.map((c) => c.name)
		expect(names).toEqual(['compact', 'clear', 'review-code'])
		expect(manager.sessionCommands[0].description).toBe(
			'Summarize the conversation to free up context'
		)
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
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [], [])
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
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [], [])
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
			config.abortController.abort()
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
			config.abortController.abort()
			throw new Error('aborted')
		})

		manager.instructions = 'think hard'
		await manager.sendRequest()

		// Nothing usable was produced → treat as unsent: roll the turn back out
		// (user message + stuck-open reasoning bubble) and restore the composer.
		expect(manager.messages.some((m) => m.role === 'assistant')).toBe(false)
		expect(manager.displayMessages.some((m) => m.role === 'assistant')).toBe(false)
		expect(manager.displayMessages.some((m) => m.role === 'user')).toBe(false)
		expect(restoreInstructions).toHaveBeenCalledWith('think hard', [], [])
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
			config.abortController.abort()
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
			config.abortController.abort()
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

describe('AIChatManager background job completion', () => {
	const completed = (over: Record<string, unknown> = {}) =>
		({
			type: 'CompletedJob',
			id: 'job-1',
			success: true,
			canceled: false,
			result: [{ n: 1 }],
			duration_ms: 1234,
			logs: 'ran',
			...over
		}) as any

	beforeEach(() => {
		localStorage.clear()
		vi.clearAllMocks()
		mocks.getCurrentModel.mockReturnValue({ provider: 'openai', model: 'gpt-4o' })
	})

	// Drive a registered+detached job to completion through the public poller entry
	// (refreshBackgroundJobs polls immediately) and wait until the poller reports it.
	async function completeDetachedJob(manager: AIChatManager) {
		manager.markJobDetached('job-1')
		manager.refreshBackgroundJobs()
		await vi.waitFor(() => expect(manager.backgroundJobs[0]?.reported).toBe(true))
	}

	// A ChatJob carrying only its serializable resultFormat (no in-memory closure) —
	// exactly the shape a job has after being rehydrated from IndexedDB on reload.
	const datatableJob = {
		jobId: 'job-1',
		toolCallId: 'tc-1',
		kind: 'script' as const,
		label: 'SQL · main',
		workspace: 'ws',
		resultFormat: { kind: 'datatable' as const, datatableName: 'main' }
	}

	it('reconstructs the datatable result contract from the persisted resultFormat', async () => {
		const manager = new AIChatManager()
		manager.registerJob(datatableJob)
		const applyToolStatus = vi.spyOn(manager, 'applyToolStatus')
		mocks.getJob.mockResolvedValue(completed({ result: [{ n: 1 }, { n: 2 }] }))

		await completeDetachedJob(manager)

		// No live closure is involved: the descriptor alone reshapes both the tool card
		// and the model note, so a job that detached and survived a reload still reports
		// the SQL contract (row count + shaped rows) rather than generic job output.
		expect(applyToolStatus).toHaveBeenCalledWith('tc-1', {
			content: 'Query returned 2 row(s)',
			result: JSON.stringify([{ n: 1 }, { n: 2 }], null, 2)
		})
		expect(manager.pendingJobNotes).toHaveLength(1)
		expect(manager.pendingJobNotes[0]).toContain('"rowCount": 2')
	})

	it('skips reconstruction and emits no note for a canceled detached job', async () => {
		const manager = new AIChatManager()
		manager.registerJob(datatableJob)
		const applyToolStatus = vi.spyOn(manager, 'applyToolStatus')
		mocks.getJob.mockResolvedValue(completed({ success: false, canceled: true }))

		await completeDetachedJob(manager)

		// A user cancel isn't a result to shape or a completion to announce.
		expect(manager.pendingJobNotes).toHaveLength(0)
		expect(manager.backgroundJobs[0]?.status).toBe('canceled')
		expect(applyToolStatus).toHaveBeenCalledWith('tc-1', {
			content: 'Background job canceled',
			logs: expect.anything()
		})
	})

	it('falls back to the generic note when the job has no resultFormat', async () => {
		const manager = new AIChatManager()
		manager.registerJob({
			jobId: 'job-1',
			toolCallId: 'tc-1',
			kind: 'script',
			label: 'run',
			workspace: 'ws'
		})
		mocks.getJob.mockResolvedValue(completed())

		await completeDetachedJob(manager)

		expect(manager.pendingJobNotes).toHaveLength(1)
		expect(manager.pendingJobNotes[0]).toContain('Background job job-1 for "run" succeeded')
	})
})
