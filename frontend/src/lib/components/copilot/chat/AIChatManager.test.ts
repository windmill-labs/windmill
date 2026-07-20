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

	// The real composer reports whether it took the restore (an occupied one declines);
	// default to an empty composer, which always takes it.
	function createInputMock() {
		return {
			prependText: vi.fn().mockReturnValue(false),
			restoreInstructions: vi.fn().mockReturnValue(true),
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
		expect(input.prependText).toHaveBeenCalledWith('line one\nline two', [], [])
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

		// The auto-sent turn must carry the whole submitted message — queueing must
		// not send the text alone and drop its images.
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

	it('an edit resends the edited context, a bare retry the original', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		const cm = manager.contextManager
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		const chipA = cm.getSelectedContext()[0]
		cm.setSelectedDomElement({ selector: 'div.b', appPath: 'f/app', tagName: 'div' })
		const chipB = cm.getSelectedContext()[0]
		cm.clearSelectedDomElements()

		const seed = () => {
			manager.displayMessages = [
				{ role: 'user', content: 'style it', index: 0, contextElements: [chipA] },
				{ role: 'assistant', content: 'ok' }
			]
			manager.messages = [
				{ role: 'user', content: 'style it' },
				{ role: 'assistant', content: 'ok' }
			]
		}
		const sentChipSelectors = () =>
			(manager.displayMessages.find((m) => m.role === 'user')?.contextElements ?? [])
				.filter((c) => c.type === 'app_dom_selector')
				.map((c) => c.selector)

		// Edit swapped the chip A → B in the edit box: the resend carries B, not A.
		seed()
		manager.restartGeneration(0, 'style it', undefined, undefined, [chipB])
		await vi.waitFor(() => expect(sentChipSelectors()).toEqual(['div.b']))

		// A bare retry passes no edited context and falls back to the original A.
		seed()
		manager.restartGeneration(0)
		await vi.waitFor(() => expect(sentChipSelectors()).toEqual(['div.a']))

		// An edit/retry replays context that was consumed on its original send, so it
		// must not touch the composer's own live selection — even when it holds the
		// very same chip.
		seed()
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		manager.restartGeneration(0)
		await vi.waitFor(() => expect(sentChipSelectors()).toEqual(['div.a']))
		expect(
			cm
				.getSelectedContext()
				.filter((c) => c.type === 'app_dom_selector')
				.map((c) => c.selector)
		).toEqual(['div.a'])
	})

	// The loop, not the send, owns the vision strip: it re-applies it per iteration
	// for whatever model that iteration runs on, so a mid-loop switch in either
	// direction sees the right view. A copy stripped at send time could never be
	// un-stripped when the user switches text-only → vision during the turn.
	it('passes the full history to the loop even on a text-only model', async () => {
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
		expect(anyImage).toBe(true)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// Empty instructions are a valid image-only send; they must override, not
	// keep, text a failed or cancelled earlier turn left in this.instructions.
	it('does not attach stale instructions to an image-only send', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.instructions = 'text from a failed earlier turn'

		await manager.sendRequest({ instructions: '', images: [img('a')] })

		const sent = mocks.runChatLoop.mock.calls[0][0].messages.at(-1)
		const text = Array.isArray(sent.content)
			? sent.content
					.filter((p: any) => p.type === 'text')
					.map((p: any) => p.text)
					.join('\n')
			: sent.content
		expect(text).not.toContain('text from a failed earlier turn')
	})

	// The failing request may have used the model selected at send time, not the
	// currently selected one — a mid-flight switch must not stop its id being
	// excluded from the rejection match.
	it('does not strip images when the error echoes the send-time model after a mid-flight switch', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.tryGetCurrentModel.mockReturnValue({
			provider: 'openrouter',
			model: 'meta-llama/llama-3.2-90b-vision-instruct'
		})
		mocks.runChatLoop.mockImplementation(async () => {
			// the user switches models while the request is in flight...
			mocks.tryGetCurrentModel.mockReturnValue({ provider: 'openai', model: 'gpt-4o' })
			// ...and the in-flight model fails with an unrelated error echoing its id
			throw new Error('429 Rate limit reached for meta-llama/llama-3.2-90b-vision-instruct')
		})

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(true)
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

	// Vision model ids often contain the rejection subject words themselves, and
	// providers echo the id in unrelated errors. A rate limit must not read as an
	// image rejection just because the model is called "...-vision-instruct" —
	// the strip it would trigger is permanent (retry refuses the transcript copy).
	it('keeps the image when a transient error merely echoes a vision model id', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.tryGetCurrentModel.mockReturnValue({
			provider: 'openrouter',
			model: 'meta-llama/llama-3.2-90b-vision-instruct'
		})
		mocks.runChatLoop.mockImplementation(async () => {
			throw new Error('429 Rate limit reached for model meta-llama/llama-3.2-90b-vision-instruct')
		})

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(true)
	})

	// The refused image is not always this turn's attachment: an unlisted
	// text-only model receives the full history, so a screenshot follow-up or an
	// earlier upload can be the part it chokes on. Without the strip, every later
	// send resubmits it and fails identically — a wedge with no self-correction.
	it('removes historical images from history when the provider rejects them on a text turn', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		// e.g. a take_screenshot follow-up from an earlier turn
		manager.messages = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'Screenshot of the app preview:' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,SHOT' } }
				] as any
			},
			{ role: 'assistant', content: 'looks good' }
		]
		mocks.runChatLoop.mockImplementation(async () => {
			throw new Error('400 this model does not support image input')
		})

		await manager.sendRequest({ instructions: 'plain text follow-up' })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(false)
		expect(mocks.sendUserToast).toHaveBeenCalledWith(
			expect.stringContaining('could not read the attached image'),
			true
		)
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

	// The wire format has no filename; a retried/edited image must recover it
	// from the bubble's entry — an unnamed resend would downgrade an image-only
	// chat's filename-derived history title to the generic fallback.
	it('storedImages recovers attachment names from the transcript bubble', () => {
		const manager = createManager()
		manager.messages = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'look' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,FULL' } }
				] as any
			}
		]
		manager.displayMessages = [
			{
				role: 'user',
				content: 'look',
				index: 0,
				images: [
					{ dataUrl: 'data:image/png;base64,FULL', mediaType: 'image/png', name: 'mockup.png' }
				]
			} as any
		]

		expect(manager.storedImages(0)).toEqual([
			{ dataUrl: 'data:image/png;base64,FULL', mediaType: 'image/png', name: 'mockup.png' }
		])
	})

	// Drop-oldest removes the API counterpart but the transcript keeps the bubble.
	// Its restart index must not alias to a surviving message, or retrying/editing
	// the dropped prompt would silently attach that other turn's images.
	it("does not serve another turn's images for a message dropped by drop-oldest compaction", () => {
		const manager = createManager()
		manager.messages = [
			{ role: 'user', content: 'old prompt' },
			{ role: 'assistant', content: 'old answer' },
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'new prompt' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,NEW' } }
				] as any
			},
			{ role: 'assistant', content: 'new answer' }
		]
		manager.displayMessages = [
			{ role: 'user', content: 'old prompt', index: 0 },
			{ role: 'assistant', content: 'old answer' },
			{ role: 'user', content: 'new prompt', index: 2, images: [img('thumb')] },
			{ role: 'assistant', content: 'new answer' }
		]

		// frees the first turn (user + assistant), keeps the image-bearing one
		manager.compactOldestMessages(1)

		expect(manager.messages.length).toBe(2)
		// the dropped message resolves no images...
		expect(manager.storedImages(0)).toBeUndefined()
		// ...while the surviving one still resolves its own
		expect(manager.storedImages(2)?.[0]?.dataUrl).toBe('data:image/png;base64,NEW')
	})

	// Enter with an image but no text must send, not silently discard the image
	// (the input clears itself optimistically, so a bail on empty text loses it).
	it('sends an image-only message in GLOBAL mode', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL

		await manager.sendRequest({ instructions: '', images: [img('a')] })

		expect(mocks.runChatLoop).toHaveBeenCalled()
		const sent = mocks.runChatLoop.mock.calls[0][0].messages.at(-1)
		const hasImage =
			Array.isArray(sent.content) && sent.content.some((p: any) => p.type === 'image_url')
		expect(hasImage).toBe(true)
	})

	it('still ignores a send with no text and no images', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL

		await manager.sendRequest({ instructions: '' })

		expect(mocks.runChatLoop).not.toHaveBeenCalled()
	})

	// With no text, dropping the images leaves nothing to send — they must go back
	// to the composer (which already cleared itself optimistically), not vanish.
	it('restores the images when an image-only send meets a text-only model', async () => {
		replyWith('done')
		const input = createInputMock()
		const manager = createManager(input)
		manager.mode = AIMode.GLOBAL
		mocks.tryGetCurrentModel.mockReturnValue({
			provider: 'groq',
			model: 'llama-3.3-70b-versatile'
		})

		await manager.sendRequest({ instructions: '', images: [img('a')] })

		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		expect(input.prependText).toHaveBeenCalledWith('', [img('a')], [])
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// A refused queued draft is the caller's to restore (it re-queues on false) —
	// a composer restore on top would leave the same attachment in both places.
	it('does not double-restore a queued image-only draft refused by a text-only model', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.mode = AIMode.GLOBAL
		mocks.tryGetCurrentModel.mockReturnValue({
			provider: 'groq',
			model: 'llama-3.3-70b-versatile'
		})

		const accepted = await manager.sendRequest({
			instructions: '',
			images: [img('a')],
			queued: true
		})

		expect(accepted).toBe(false)
		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		expect(input.prependText).not.toHaveBeenCalled()
		expect(input.restoreInstructions).not.toHaveBeenCalled()
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// A→B→C: the loop can run an iteration on a model that is neither the
	// send-time one (A) nor the currently-selected one (C) by the time the
	// failure is classified. The failing iteration's id (B) must be excluded
	// from the rejection match too.
	it('does not strip images when the error echoes an intermediate model (A→B→C)', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		const a = { provider: 'openai', model: 'gpt-4o' }
		const b = { provider: 'openrouter', model: 'meta-llama/llama-3.2-90b-vision-instruct' }
		const c = { provider: 'anthropic', model: 'claude-sonnet-4-6' }
		mocks.getCurrentModel.mockReturnValue(a)
		mocks.tryGetCurrentModel.mockReturnValue(a)
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			// an iteration starts on B...
			await config.onBeforeIteration?.([], config.helpers, b)
			// ...the user switches to C while B's request is in flight...
			mocks.getCurrentModel.mockReturnValue(c)
			mocks.tryGetCurrentModel.mockReturnValue(c)
			// ...and B fails with an unrelated error echoing its id
			throw new Error('429 Rate limit reached for meta-llama/llama-3.2-90b-vision-instruct')
		})

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(true)
		mocks.getCurrentModel.mockReturnValue(model)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// The Responses converter sends images as input_image parts, and '_' is a
	// word character — the whole-word regex must still catch that spelling.
	it('recovers when the provider rejects the input_image content part', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.runChatLoop.mockRejectedValue(
			new Error("400 Invalid value: content part type 'input_image' is not supported")
		)

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(false)
	})

	// The composer stays mounted across a mode switch, so chips attached in
	// GLOBAL can ride a send in any mode — they must be restored, not dropped.
	it('refuses and restores an image-bearing send outside GLOBAL mode', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.mode = AIMode.NAVIGATOR

		const pending = manager.sendRequest({ instructions: 'find it', images: [img('a')] })
		// The composer clears itself synchronously right after calling sendRequest:
		// a restore issued before that point would be wiped by the clear.
		expect(input.restoreInstructions).not.toHaveBeenCalled()
		const accepted = await pending

		expect(accepted).toBe(false)
		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		expect(input.restoreInstructions).toHaveBeenCalledWith('find it', [], [img('a')], [])
	})

	// A refused queued draft is the caller's to restore (it re-queues on false) —
	// a composer restore on top would duplicate it.
	it('does not double-restore a queued image draft refused outside GLOBAL mode', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.mode = AIMode.NAVIGATOR

		const accepted = await manager.sendRequest({
			instructions: 'queued one',
			images: [img('a')],
			queued: true
		})

		expect(accepted).toBe(false)
		expect(input.restoreInstructions).not.toHaveBeenCalled()
		expect(input.prependText).not.toHaveBeenCalled()
	})

	// "provisioning"/"provisioned" contain the word "vision" — a transient
	// capacity error must not be classified as an image rejection.
	it('does not strip images on a provisioning error', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		mocks.runChatLoop.mockRejectedValue(
			new Error('503 model provisioning failed, please retry later')
		)

		await manager.sendRequest({ instructions: 'look at this', images: [img('a')] })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(true)
	})

	// A turn can start on a known text-only model (send-time flag says "no images
	// go out") and switch mid-loop to an UNLISTED blind model whose iteration does
	// carry the history's images. When that model rejects them, recovery must fire
	// — the send-time flag alone would skip it and wedge every later send.
	it('recovers when a turn starts text-only but an unlisted blind model rejects mid-loop', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.messages = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'earlier' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,OLD' } }
				] as any
			},
			{ role: 'assistant', content: 'ok' }
		]
		const knownBlind = { provider: 'groq', model: 'llama-3.3-70b-versatile' }
		const unlistedBlind = { provider: 'customai', model: 'my-internal-llm' }
		mocks.getCurrentModel.mockReturnValue(knownBlind)
		mocks.tryGetCurrentModel.mockReturnValue(knownBlind)
		mocks.runChatLoop.mockImplementation(async (config: any) => {
			// mid-loop switch to a model the deny-list doesn't know...
			mocks.getCurrentModel.mockReturnValue(unlistedBlind)
			mocks.tryGetCurrentModel.mockReturnValue(unlistedBlind)
			await config.onBeforeIteration?.([], config.helpers, unlistedBlind)
			// ...its request carries the images and the provider rejects them
			throw new Error('400 this model does not support image input')
		})

		await manager.sendRequest({ instructions: 'plain follow-up' })

		const stillThere = manager.messages.some(
			(m: any) => Array.isArray(m.content) && m.content.some((p: any) => p.type === 'image_url')
		)
		expect(stillThere).toBe(false)
		mocks.getCurrentModel.mockReturnValue(model)
		mocks.tryGetCurrentModel.mockReturnValue(model)
	})

	// Images evicted from requests by the byte bound must not keep their full
	// data URLs in stored history: provider-reported usage excludes them, so
	// compaction would never prune them and every save re-clones the payload.
	// The bubble and the API message must share the exact same data URL — the
	// history's blob store dedups them to a single record on save, so a
	// transcript-side copy (e.g. a downscale) would double the stored bytes.
	it('sends and displays the same image copy', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL

		await manager.sendRequest({ instructions: 'look', images: [img('a')] })

		const bubble = manager.displayMessages.find((m) => m.role === 'user') as any
		const sent = mocks.runChatLoop.mock.calls[0][0].messages.at(-1)
		const sentUrl = sent.content.find((p: any) => p.type === 'image_url').image_url.url
		expect(bubble.images[0].dataUrl).toBe(sentUrl)
	})

	it('queues an image-only message and restores it on dequeue', () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.queueMessage('', [img('a')])

		expect(manager.queuedMessage).toBe('')
		expect(manager.queuedImages).toEqual([img('a')])

		manager.dequeueMessage()

		expect(manager.queuedImages).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('', [img('a')], [])
	})

	it('queues a file-only message and restores it on dequeue', () => {
		const input = createInputMock()
		const manager = createManager(input)
		const file = { name: 'notes.md', content: 'hello' }
		manager.queueMessage('', [], undefined, [file])

		expect(manager.queuedMessage).toBe('')
		expect(manager.queuedFiles).toEqual([file])

		manager.dequeueMessage()

		expect(manager.queuedFiles).toEqual([])
		expect(input.prependText).toHaveBeenCalledWith('', [], [file])
	})

	// While editing an earlier message the bottom composer and the edit box are
	// both mounted. Each enforces MAX_CONVERSATION_FILE_BYTES at attach time, so
	// each must see the other's stage or two attaches could each spend the full
	// budget and overflow the persisted transcript.
	it('counts every other live composer stage in the attachment budget', () => {
		const manager = new AIChatManager()
		manager.displayMessages = [
			{ role: 'user', content: 'edited', files: [{ name: 'a.md', content: 'X'.repeat(300) }] },
			{ role: 'user', content: 'kept', files: [{ name: 'b.md', content: 'Y'.repeat(500) }] }
		] as any

		// Bottom composer staged 4MB; edit box (editing message 0) staged 900KB.
		manager.setComposerStaged('main', null, 4_000_000)
		manager.setComposerStaged('edit', 0, 900_000)

		// From the bottom composer: message 0 is skipped (its editor's stage stands
		// in for it), message 1 counts, and the edit box's 900KB is visible.
		expect(manager.attachmentBytesExcluding('main')).toBe(500 + 900_000)
		// From the edit box: message 0 skipped, message 1 counts, bottom's 4MB visible.
		expect(manager.attachmentBytesExcluding('edit')).toBe(500 + 4_000_000)

		manager.clearComposerStaged('edit')
		expect(manager.attachmentBytesExcluding('main')).toBe(300 + 500)
	})

	// The edit box unmounts (dropping its stage) the instant the user submits, but
	// restartGeneration then awaits registry sync + upkeep before the resent bubble
	// lands in the transcript. During that gap the resent files must stay reserved,
	// or the bottom composer could attach into the temporary headroom and overflow.
	it('reserves resent files across the restartGeneration gap', async () => {
		replyWith('done')
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		const fileRow = { name: 'a.md', content: 'X'.repeat(3000) }
		manager.displayMessages = [{ role: 'user', content: 'orig', files: [fileRow], index: 0 }] as any
		manager.messages = [{ role: 'user', content: 'orig' }] as any

		// refreshFolders runs inside sendRequest AFTER the edited message was sliced
		// out but BEFORE the resent bubble is installed — the one moment the gap is
		// open. The reservation must cover the resent bytes there.
		let observed: number | undefined
		vi.spyOn(manager.attachedFiles, 'refreshFolders').mockImplementation(async () => {
			observed = manager.attachmentBytesExcluding('probe')
		})

		await manager.restartGeneration(0)
		// Drain the resend turn fully (its runChatLoop resolves immediately) so no
		// async work bleeds into a later test's shared-mock call counts.
		for (let i = 0; i < 50; i++) await new Promise((r) => setTimeout(r, 0))

		expect(observed).toBe(3000)
		// Once the turn installs the bubble, the reservation is released — the
		// transcript now accounts those bytes on its own.
		expect(manager.attachmentBytesExcluding('probe')).toBe(3000)
	})

	// A local command (/clear, /compact) consumes the send and returns before a
	// bubble installs, so an edit resolved to one must not strand its reservation.
	it('releases the resend reservation when an edit resolves to a local command', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.isSessionChat = true
		vi.spyOn(manager, 'compactManually').mockResolvedValue()
		const fileRow = { name: 'a.md', content: 'X'.repeat(2000) }
		manager.displayMessages = [{ role: 'user', content: 'orig', files: [fileRow], index: 0 }] as any
		manager.messages = [{ role: 'user', content: 'orig' }] as any

		await manager.restartGeneration(0, '/compact')
		for (let i = 0; i < 20; i++) await new Promise((r) => setTimeout(r, 0))

		// No stranded reservation: the abandoned resend charges nothing.
		expect(manager.attachmentBytesExcluding('probe')).toBe(0)
	})

	// The reservation is keyed per resend, so a normal (or concurrent) send that
	// carries no token must never release a resend reservation it doesn't own.
	it('a normal send does not release another send resend reservation', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		// An in-flight resend owns this reservation.
		manager.setComposerStaged('resend:other', null, 4000)

		// A normal send that bails early (empty draft) carries no reservation key.
		await manager.sendRequest({ instructions: '   ' })

		expect(manager.attachmentBytesExcluding('probe')).toBe(4000)
	})

	// Drop-oldest compaction (summary fallback) removes API messages without a
	// summary, so a folded message's `## ATTACHED FILES` reference no longer reaches
	// the model. Its file (index < 0) must be advertised through the roster instead.
	it('flags message files whose referencing message was dropped by compaction', () => {
		const manager = new AIChatManager()
		manager.displayMessages = [
			{ role: 'user', content: 'a', index: -1, files: [{ name: 'dropped.md', content: 'x' }] },
			{ role: 'user', content: 'b', index: 0, files: [{ name: 'live.md', content: 'y' }] },
			// Referenced by BOTH a dropped and a surviving message → still visible, not orphaned.
			{ role: 'user', content: 'c', index: -1, files: [{ name: 'shared.md', content: 'z' }] },
			{ role: 'user', content: 'd', index: 1, files: [{ name: 'shared.md', content: 'z' }] }
		] as any

		expect([...manager.orphanedMessageFileNames()]).toEqual(['dropped.md'])
	})

	// A summary carries its folded files' reference on its own API message; if a
	// later drop-oldest (summary fallback) removes that message, the reference is
	// gone and the files must move to the roster like any other orphan.
	it('orphans summary-carried files when drop-oldest removes the summary', () => {
		const manager = new AIChatManager()
		manager.messages = [
			{ role: 'user', content: 'summary api message' },
			{ role: 'user', content: 'tail' }
		] as any
		manager.displayMessages = [
			{ role: 'summary', content: 's', index: 0, files: [{ name: 'folded.md', content: 'x' }] },
			{ role: 'user', content: 'tail', index: 1 }
		] as any

		// Summary API message present → its files are still referenced.
		expect([...manager.orphanedMessageFileNames()]).toEqual([])

		// Drop-oldest removes the summary's API message and re-bases indices.
		manager.compactOldestMessages(1)

		expect([...manager.orphanedMessageFileNames()]).toEqual(['folded.md'])
	})

	// An edit is not committed until send, so cancelling it returns the message's
	// persisted attachments. Charging only the (possibly emptied) edit stage would
	// hand the bottom composer headroom that vanishes on cancel — remove the files
	// in the editor, fill the bottom draft, cancel, and the transcript overflows.
	it('charges an edited message at its persisted size until the edit commits', () => {
		const manager = new AIChatManager()
		manager.displayMessages = [
			{ role: 'user', content: 'big', files: [{ name: 'a.md', content: 'X'.repeat(4000) }] }
		] as any

		// Edit box mounted on message 0 with its attachment removed (stage 0):
		// the bottom composer must still see the 4000 persisted bytes.
		manager.setComposerStaged('edit', 0, 0)
		expect(manager.attachmentBytesExcluding('main')).toBe(4000)

		// Once the editor stages more than the original, the larger figure wins.
		manager.setComposerStaged('edit', 0, 9000)
		expect(manager.attachmentBytesExcluding('main')).toBe(9000)
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

	it('restores consumed DOM selector chips when a turn is cancelled before output', async () => {
		const manager = createManager(createInputMock())
		manager.mode = AIMode.GLOBAL
		manager.contextManager.setSelectedDomElement({
			selector: 'div.card',
			appPath: 'f/app',
			tagName: 'div'
		})
		// The chip is consumed on send; while the turn streams the user selects a
		// different element, then cancels before any usable output (rollback path).
		mocks.runChatLoop.mockImplementationOnce(async ({ abortController }: any) => {
			manager.contextManager.addSelectedDomElement({
				selector: 'div.other',
				appPath: 'f/app',
				tagName: 'div'
			})
			abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		await manager.sendRequest({ instructions: 'make it red' })

		// Rollback restores THIS turn's chip and replaces the chip selected mid-stream,
		// so the restored draft stays coherent (its instruction targets div.card only).
		const chips = manager.contextManager
			.getSelectedContext()
			.filter((c) => c.type === 'app_dom_selector')
		expect(chips.map((c) => c.selector)).toEqual(['div.card'])
	})

	it('restores a dequeued inline prompt’s pinned DOM context, replacing the live selection', () => {
		const manager = createManager(createInputMock())
		const cm = manager.contextManager
		// Prompt A was queued with its own element pinned.
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		manager.queueMessage('style A', [], [...cm.getSelectedContext()])
		// The user then selects B in the live preview.
		cm.setSelectedDomElement({ selector: 'div.b', appPath: 'f/app', tagName: 'div' })

		// Returning the queued draft to the composer must restore A's context, not
		// leave B's live selection (which would retarget the restored prompt).
		manager.dequeueMessage()

		const chips = cm.getSelectedContext().filter((c) => c.type === 'app_dom_selector')
		expect(chips.map((c) => c.selector)).toEqual(['div.a'])
	})

	// Restoration is only coherent when the text it belongs to actually lands in the
	// composer. Both cases below leave another draft sitting there, so replacing its
	// chips would silently retarget an instruction the user is still writing.
	it('leaves an occupied composer’s DOM context alone when it declines a cancelled prompt', async () => {
		const input = createInputMock()
		// The user typed a B-scoped draft during the stream, so the composer keeps it
		// and declines the cancelled prompt's text.
		input.restoreInstructions.mockReturnValue(false)
		const manager = createManager(input)
		manager.mode = AIMode.GLOBAL
		const cm = manager.contextManager
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		mocks.runChatLoop.mockImplementationOnce(async ({ abortController }: any) => {
			cm.addSelectedDomElement({ selector: 'div.b', appPath: 'f/app', tagName: 'div' })
			abortController.abort('user_cancelled')
			throw new Error('aborted')
		})

		await manager.sendRequest({ instructions: 'style A' })

		const chips = cm.getSelectedContext().filter((c) => c.type === 'app_dom_selector')
		expect(chips.map((c) => c.selector)).toEqual(['div.b'])
	})

	it('keeps both drafts’ chips when a dequeued prompt is prepended onto an existing draft', () => {
		const input = createInputMock()
		// prependText merged the queued text on top of a draft already in the composer.
		input.prependText.mockReturnValue(true)
		const manager = createManager(input)
		const cm = manager.contextManager
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		manager.queueMessage('style A', [], [...cm.getSelectedContext()])
		cm.setSelectedDomElement({ selector: 'div.b', appPath: 'f/app', tagName: 'div' })

		manager.dequeueMessage()

		// Both instructions now share one composer, so both elements stay in scope.
		const chips = cm.getSelectedContext().filter((c) => c.type === 'app_dom_selector')
		expect(chips.map((c) => c.selector).sort()).toEqual(['div.a', 'div.b'])
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

	// The composer clears itself optimistically on send, so a preflight failure
	// must put the whole draft back — images can't just be re-dropped from memory.
	it('restores text and images to the composer when beforeSend rejects a direct send', async () => {
		const input = createInputMock()
		const manager = createManager(input)
		manager.mode = AIMode.GLOBAL
		manager.beforeSend = vi.fn().mockRejectedValue(new Error('workspace fork failed'))

		const accepted = await manager.sendRequest({ instructions: 'look', images: [img('a')] })

		expect(accepted).toBe(false)
		expect(mocks.runChatLoop).not.toHaveBeenCalled()
		expect(input.restoreInstructions).toHaveBeenCalledWith('look', [], [img('a')], [])
		// the optimistic bubble is rolled back
		expect(manager.displayMessages).toHaveLength(0)
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

	it('re-bases display message indices, marking fully-compacted ones negative', () => {
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
		// A dropped message's index goes negative rather than clamping to 0:
		// 0 would alias it to the first surviving message, and storedImages
		// would serve that message's images to a retry of this one.
		expect(manager.displayMessages.map((m) => ('index' in m ? m.index : undefined))).toEqual([
			-2,
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

	it('carries folded-away message files on the summary', async () => {
		mocks.getCurrentModel.mockReturnValue(gpt4oModel)
		mocks.tryGetCurrentModel.mockReturnValue(gpt4oModel)
		mocks.getNonStreamingCompletion.mockResolvedValue(
			'<analysis>s</analysis><summary>SUM</summary>'
		)
		const manager = new AIChatManager()
		seedForSummary(manager)
		const file = { name: 'notes.md', content: 'hello' }
		// The identical file on TWO folded turns (identical content registers under
		// one name) must carry as ONE summary entry.
		manager.displayMessages = manager.displayMessages.map((m, i) =>
			(i === 0 || i === 2) && m.role === 'user' ? { ...m, files: [file] } : m
		)

		await manager.sendRequest()

		// The summary display message carries the folded-away file once, the API
		// summary references it as still-readable, and the registry keeps its row.
		expect(manager.displayMessages[0]).toMatchObject({ role: 'summary', files: [file] })
		const sent = mocks.runChatLoop.mock.calls[mocks.runChatLoop.mock.calls.length - 1][0].messages
		expect(sent[0].content).toContain('notes.md')
		expect(sent[0].content).toContain('read_file')
		expect(manager.attachedFiles.messageAttached.map((f) => f.name)).toEqual(['notes.md'])
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
		// The summarizer's output must stay capped: without it the model default
		// applies and the Anthropic SDK rejects the non-streaming call pre-flight.
		expect(mocks.getNonStreamingCompletion.mock.calls[0][2]).toEqual({ maxTokensCap: 8000 })

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
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [], [], [])
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
		expect(restoreInstructions).toHaveBeenCalledWith('do a thing', [], [], [])
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
		expect(restoreInstructions).toHaveBeenCalledWith('think hard', [], [], [])
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

describe('DOM selector chips scoped by app path', () => {
	const domChips = (manager: AIChatManager) =>
		manager.contextManager.getSelectedContext().filter((c) => c.type === 'app_dom_selector')

	it('keeps same-selector chips from different apps and removes only the scoped one', () => {
		const manager = new AIChatManager()
		const cm = manager.contextManager
		const base = { selector: 'div.card', tagName: 'div' }
		cm.addSelectedDomElement({ ...base, appPath: 'f/app/a' })
		cm.addSelectedDomElement({ ...base, appPath: 'f/app/b' })
		// Same selector, different apps: both survive (dedup is per app path).
		expect(domChips(manager)).toHaveLength(2)

		// A selector-only removal would wipe both; scoping by appPath keeps app A's.
		cm.removeSelectedDomElement('div.card', 'f/app/b')
		const remaining = domChips(manager)
		expect(remaining).toHaveLength(1)
		expect(remaining[0].appPath).toBe('f/app/a')
	})

	it("a scoped clear (preview rebuild) drops only that app's chips", () => {
		const manager = new AIChatManager()
		const cm = manager.contextManager
		cm.addSelectedDomElement({ selector: 'h1', appPath: 'f/app/a', tagName: 'h1' })
		cm.addSelectedDomElement({ selector: 'button', appPath: 'f/app/b', tagName: 'button' })

		// App A rebuilding must not wipe app B's active selection.
		cm.clearSelectedDomElements('f/app/a')
		const remaining = domChips(manager)
		expect(remaining).toHaveLength(1)
		expect(remaining[0].appPath).toBe('f/app/b')

		// An unscoped clear (post-send / foreign reset) still drops everything.
		cm.clearSelectedDomElements()
		expect(domChips(manager)).toHaveLength(0)
	})

	it('unions DOM chips across inline prompts queued during one stream', () => {
		const manager = new AIChatManager()
		const cm = manager.contextManager
		cm.setSelectedDomElement({ selector: 'div.a', appPath: 'f/app', tagName: 'div' })
		const snapA = [...cm.getSelectedContext()]
		cm.setSelectedDomElement({ selector: 'div.b', appPath: 'f/app', tagName: 'div' })
		const snapB = [...cm.getSelectedContext()]

		// Two element-scoped inline prompts queued while a turn streams. The earlier
		// element's chip must survive so its instruction isn't retargeted to the later one.
		manager.queueMessage('make A red', [], snapA)
		manager.queueMessage('make B bigger', [], snapB)

		const queuedSelectors = (manager.queuedContext ?? [])
			.filter((c) => c.type === 'app_dom_selector')
			.map((c) => c.selector)
			.sort()
		expect(queuedSelectors).toEqual(['div.a', 'div.b'])
	})
})
