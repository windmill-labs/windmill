import type { AIProviderModel, ScriptLang } from '$lib/gen/types.gen'
import { WorkspaceService } from '$lib/gen'
import type { FlowOptions, ScriptOptions } from './ContextManager.svelte'
import {
	flowTools,
	prepareFlowSystemMessage,
	prepareFlowUserMessage,
	type FlowAIChatHelpers
} from './flow/core'
import {
	getAppTools,
	prepareAppSystemMessage,
	prepareAppUserMessage,
	type AppAIChatHelpers
} from './app/core'
import ContextManager from './ContextManager.svelte'
import HistoryManager from './HistoryManager.svelte'
import {
	type DisplayMessage,
	type Tool,
	type ToolCallbacks,
	type ToolDisplayMessage
} from './shared'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam
} from 'openai/resources/chat/completions.mjs'
import {
	prepareInlineChatSystemPrompt,
	prepareScriptSystemMessage,
	prepareScriptTools
} from './script/core'
import type { ScriptLintResult } from './shared'
import { navigatorTools, prepareNavigatorSystemMessage } from './navigator/core'
import { loadApiTools } from './api/apiTools'
import { prepareScriptUserMessage } from './script/core'
import { prepareNavigatorUserMessage } from './navigator/core'
import { sendUserToast } from '$lib/toast'
import { getModelContextWindow, workspaceAIClients } from '../lib'
import { dfs } from '$lib/components/flows/previousResults'
import { getStringError } from './utils'
import type { FlowModuleState, FlowState } from '$lib/components/flows/flowState'
import type { CurrentEditor, ExtendedOpenFlow } from '$lib/components/flows/types'
import { untrack } from 'svelte'
import { get } from 'svelte/store'
import { BROWSER } from 'esm-env'
import { workspaceStore, type DBSchemas } from '$lib/stores'
import { askTools, prepareAskSystemMessage, prepareAskUserMessage } from './ask/core'
import { chatState, DEFAULT_SIZE, triggerablesByAi } from './sharedChatState.svelte'
import {
	createAppBackendRunnableContextElement,
	createAppFrontendFileContextElement,
	flattenDatatablesToAppContextElements,
	type ContextElement,
	type AppDatatableElement
} from './context'
import type { Selection } from 'monaco-editor'
import type AIChatInput from './AIChatInput.svelte'
import { prepareApiSystemMessage, prepareApiUserMessage } from './api/core'
import { runChatLoop } from './chatLoop'
import type { ReviewChangesOpts } from './monaco-adapter'
import { getCurrentModel, tryGetCurrentModel, getCombinedCustomPrompt } from '$lib/aiStore'
import type { WorkspaceMutationTarget } from './workspaceTools'
import { globalTools, prepareGlobalSystemMessage, prepareGlobalUserMessage } from './global/core'
import { isGlobalAiEnabled } from './global/gate'

// If the estimated token usage is greater than the model context window - the threshold, we delete the oldest message
const MAX_TOKENS_THRESHOLD_PERCENTAGE = 0.05
const MAX_TOKENS_HARD_LIMIT = 5000
const AI_AUTONOMY_MODE_STORAGE_KEY = 'ai-chat-autonomy-mode'
const LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY = 'ai-chat-yolo-mode'

export enum AIMode {
	SCRIPT = 'script',
	FLOW = 'flow',
	APP = 'app',
	NAVIGATOR = 'navigator',
	API = 'API',
	GLOBAL = 'global',
	ASK = 'ask'
}

export enum AIAutonomyMode {
	DEFAULT = 'default',
	ACCEPT_EDIT = 'acceptedit',
	YOLO = 'yolo'
}

const ALL_AI_MODES = Object.values(AIMode)
const ALL_AI_AUTONOMY_MODES = Object.values(AIAutonomyMode)
const AUTO_ACCEPT_EDIT_MODES = new Set<AIMode>([AIMode.SCRIPT, AIMode.FLOW])
const AUTO_ACCEPT_TOOL_CONFIRMATION_MODES = new Set<AIMode>([
	AIMode.SCRIPT,
	AIMode.FLOW,
	AIMode.APP,
	AIMode.GLOBAL
])

export function isAIMode(mode: unknown): mode is AIMode {
	return ALL_AI_MODES.includes(mode as AIMode)
}

export function isAIAutonomyMode(mode: unknown): mode is AIAutonomyMode {
	return ALL_AI_AUTONOMY_MODES.includes(mode as AIAutonomyMode)
}

export function supportsAutoAcceptEdits(mode: AIMode): boolean {
	return AUTO_ACCEPT_EDIT_MODES.has(mode)
}

export function supportsAutoAcceptToolConfirmations(mode: AIMode): boolean {
	return AUTO_ACCEPT_TOOL_CONFIRMATION_MODES.has(mode)
}

export function isAIModeVisible(mode: AIMode): boolean {
	return mode !== AIMode.GLOBAL || isGlobalAiEnabled()
}

export function getVisibleAIModes(): AIMode[] {
	return ALL_AI_MODES.filter(isAIModeVisible)
}

function isWorkspacePath(path: string | undefined): path is string {
	return path?.startsWith('f/') === true || path?.startsWith('u/') === true
}

function getPersistedAutonomyMode(): AIAutonomyMode {
	if (!BROWSER || typeof localStorage === 'undefined') {
		return AIAutonomyMode.ACCEPT_EDIT
	}
	const persistedMode = localStorage.getItem(AI_AUTONOMY_MODE_STORAGE_KEY)
	if (isAIAutonomyMode(persistedMode)) {
		return persistedMode
	}
	// No stored preference: default to auto-accepting edits (tool calls still
	// require confirmation; only YOLO bypasses those). Note this means users who
	// never opened the autonomy picker now start with edit auto-accept on.
	return localStorage.getItem(LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY) === 'true'
		? AIAutonomyMode.YOLO
		: AIAutonomyMode.ACCEPT_EDIT
}

function persistAutonomyMode(mode: AIAutonomyMode) {
	if (!BROWSER || typeof localStorage === 'undefined') {
		return
	}
	localStorage.setItem(AI_AUTONOMY_MODE_STORAGE_KEY, mode)
}

export class AIChatManager {
	contextManager = new ContextManager()
	historyManager = new HistoryManager()
	abortController: AbortController | undefined = undefined
	inlineAbortController: AbortController | undefined = undefined
	// Flag to skip Responses API if it's not available (e.g., Azure region doesn't support it)
	skipResponsesApi = false

	mode = $state<AIMode>(AIMode.NAVIGATOR)
	readonly isOpen = $derived(chatState.size > 0)
	savedSize = $state<number>(0)
	instructions = $state<string>('')
	pendingPrompt = $state<string>('')
	loading = $state<boolean>(false)
	currentReply = $state<string>('')
	displayMessages = $state<DisplayMessage[]>([])
	messages = $state<ChatCompletionMessageParam[]>([])
	autonomyMode = $state<AIAutonomyMode>(getPersistedAutonomyMode())
	autoAcceptEditsAvailable = $derived(supportsAutoAcceptEdits(this.mode))
	autoAcceptEditsActive = $derived(
		this.autoAcceptEditsAvailable &&
			(this.autonomyMode === AIAutonomyMode.ACCEPT_EDIT ||
				this.autonomyMode === AIAutonomyMode.YOLO)
	)
	autoAcceptToolConfirmationsAvailable = $derived(supportsAutoAcceptToolConfirmations(this.mode))
	autoAcceptToolConfirmationsActive = $derived(
		this.autonomyMode === AIAutonomyMode.YOLO && this.autoAcceptToolConfirmationsAvailable
	)
	#automaticScroll = $state<boolean>(true)
	systemMessage = $state<ChatCompletionSystemMessageParam>({
		role: 'system',
		content: ''
	})
	tools = $state<Tool<any>[]>([])
	helpers = $state<any | undefined>(undefined)

	scriptEditorOptions = $state<ScriptOptions | undefined>(undefined)
	flowOptions = $state<FlowOptions | undefined>(undefined)
	scriptEditorApplyCode = $state<
		((code: string, opts?: ReviewChangesOpts) => void | Promise<void>) | undefined
	>(undefined)
	scriptEditorShowDiffMode = $state<(() => void) | undefined>(undefined)
	scriptEditorGetLintErrors = $state<(() => ScriptLintResult) | undefined>(undefined)
	flowAiChatHelpers = $state<FlowAIChatHelpers | undefined>(undefined)
	appAiChatHelpers = $state<AppAIChatHelpers | undefined>(undefined)
	/** Datatable creation policy: enabled flag, datatable name, and optional schema */
	datatableCreationPolicy = $state<{
		enabled: boolean
		datatable: string | undefined
		schema: string | undefined
	}>({ enabled: false, datatable: undefined, schema: undefined })
	pendingNewCode = $state<string | undefined>(undefined)
	apiTools = $state<Tool<any>[]>([])
	aiChatInput = $state<AIChatInput | null>(null)
	/** Cached datatables for app context (fetched asynchronously) */
	cachedDatatables = $state<AppDatatableElement[]>([])

	private confirmationCallbacks = new Map<string, (value: boolean) => void>()
	private userQuestionCallbacks = new Map<string, (choice: string | undefined) => void>()
	private appDatatablesRefreshTimeout: ReturnType<typeof setTimeout> | undefined = undefined

	allowedModes: Record<AIMode, boolean> = $derived({
		script: this.flowAiChatHelpers === undefined && this.scriptEditorOptions !== undefined,
		flow: this.flowAiChatHelpers !== undefined,
		app: this.appAiChatHelpers !== undefined,
		navigator: true,
		ask: true,
		API: true,
		// Dev-only gate. See `./global/gate.ts` for how to enable.
		global: isAIModeVisible(AIMode.GLOBAL)
	})

	open = $derived(chatState.size > 0)

	checkTokenUsageOverLimit = (messages: ChatCompletionMessageParam[]) => {
		const estimatedTokens = messages.reduce((acc, message) => {
			// one token is ~ 4 characters
			const tokenPerCharacter = 4
			// handle content
			if (message.content) {
				acc += message.content.length / tokenPerCharacter
			}
			// Handle tool calls
			if (message.role === 'assistant' && message.tool_calls) {
				acc += JSON.stringify(message.tool_calls).length / tokenPerCharacter
			}
			return acc
		}, 0)
		const model = getCurrentModel()
		const modelContextWindow = getModelContextWindow(model.model)
		return (
			estimatedTokens >
			modelContextWindow -
				Math.max(modelContextWindow * MAX_TOKENS_THRESHOLD_PERCENTAGE, MAX_TOKENS_HARD_LIMIT)
		)
	}

	deleteOldestMessage = (messages: ChatCompletionMessageParam[], maxDepth: number = 10) => {
		if (maxDepth <= 0 || messages.length <= 1) {
			return messages
		}
		const removed = messages.shift()

		// if the removed message is an assistant with tool calls, we need to delete correspding tool response.
		if (removed?.role === 'assistant' && removed.tool_calls) {
			if (messages.length > 0 && messages[0]?.role === 'tool') {
				messages.shift()
			}
		}

		// keep deleting messages until we are under the limit
		if (this.checkTokenUsageOverLimit(messages)) {
			return this.deleteOldestMessage(messages, maxDepth - 1)
		}
		return messages
	}

	loadApiTools = async () => {
		try {
			this.apiTools = await loadApiTools()
			if (this.mode === AIMode.API) {
				this.tools = [...this.apiTools]
			}
		} catch (err) {
			console.error('Error loading api tools', err)
			this.apiTools = []
		}
	}

	// Request confirmation from user for a tool call
	requestConfirmation = (toolId: string): Promise<boolean> => {
		if (this.autoAcceptToolConfirmationsActive) {
			return Promise.resolve(true)
		}

		return new Promise((resolve) => {
			this.confirmationCallbacks.set(toolId, resolve)
		})
	}

	// Handle confirmation response for a specific tool
	handleToolConfirmation = (toolId: string, confirmed: boolean) => {
		const confirmationCallback = this.confirmationCallbacks.get(toolId)
		if (confirmationCallback) {
			confirmationCallback(confirmed)
			this.confirmationCallbacks.delete(toolId)
		}
	}

	private acceptPendingToolConfirmations = () => {
		for (const confirmationCallback of this.confirmationCallbacks.values()) {
			confirmationCallback(true)
		}
		this.confirmationCallbacks.clear()
	}

	private acceptPendingFlowEdits = (flowHelpers = this.flowAiChatHelpers) => {
		if (flowHelpers?.hasPendingChanges()) {
			flowHelpers.acceptAllModuleActions()
		}
	}

	setAutonomyMode = (mode: AIAutonomyMode) => {
		this.autonomyMode = mode
		persistAutonomyMode(mode)

		if (this.autoAcceptToolConfirmationsActive) {
			this.acceptPendingToolConfirmations()
		}
		if (this.autoAcceptEditsActive) {
			this.acceptPendingFlowEdits()
		}
	}

	setAutoAcceptToolConfirmations = (enabled: boolean) => {
		this.setAutonomyMode(enabled ? AIAutonomyMode.YOLO : AIAutonomyMode.DEFAULT)
	}

	applyScriptEditorCode = async (code: string, opts?: ReviewChangesOpts) => {
		if (this.autoAcceptEditsActive && opts?.mode === 'revert') {
			return
		}

		const effectiveOpts =
			this.autoAcceptEditsActive && (opts?.mode ?? 'apply') === 'apply'
				? ({ ...opts, mode: 'apply', applyAll: true } satisfies ReviewChangesOpts)
				: opts
		await this.scriptEditorApplyCode?.(code, effectiveOpts)
	}

	requestUserQuestion = (
		toolId: string,
		_question: { question: string; choices: string[] }
	): Promise<string | undefined> => {
		return new Promise((resolve) => {
			this.userQuestionCallbacks.set(toolId, resolve)
		})
	}

	handleUserQuestionAnswer = (toolId: string, choice: string) => {
		const callback = this.userQuestionCallbacks.get(toolId)
		if (!callback) {
			return
		}

		this.displayMessages = this.displayMessages.map((message) => {
			if (message.role === 'tool' && message.tool_call_id === toolId && message.userQuestion) {
				return {
					...message,
					content: `User answered question: ${choice}`,
					isLoading: false,
					userQuestion: {
						...message.userQuestion,
						selectedChoice: choice
					}
				}
			}
			return message
		})

		callback(choice)
		this.userQuestionCallbacks.delete(toolId)
	}

	setAiChatInput(aiChatInput: AIChatInput | null) {
		this.aiChatInput = aiChatInput
	}

	focusInput() {
		if (this.aiChatInput) {
			this.aiChatInput.focusInput()
		}
	}

	updateMode(currentMode: AIMode) {
		if (
			!this.allowedModes[currentMode] &&
			Object.keys(this.allowedModes).filter((k) => this.allowedModes[k]).length === 1
		) {
			const firstKey = Object.keys(this.allowedModes).filter((k) => this.allowedModes[k])[0]
			this.changeMode(firstKey as AIMode)
		}
	}

	private getScriptWorkspaceMutationTarget = (): WorkspaceMutationTarget => {
		const path = this.scriptEditorOptions?.path
		const workspacePath = isWorkspacePath(path) ? path : undefined
		return {
			kind: 'script',
			path: workspacePath,
			deployed:
				workspacePath !== undefined && this.scriptEditorOptions?.lastDeployedCode !== undefined
		}
	}

	private getFlowWorkspaceMutationTarget = (): WorkspaceMutationTarget => {
		return {
			kind: 'flow',
			path: this.flowOptions?.path,
			deployed:
				!!this.flowOptions?.path &&
				!!this.flowOptions.lastDeployedFlow &&
				!this.flowOptions.lastDeployedFlow.draft_only
		}
	}

	changeMode(
		mode: AIMode,
		pendingPrompt?: string,
		options?: {
			closeScriptSettings?: boolean
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
			workflowAsCode?: boolean
		}
	) {
		if (!isAIModeVisible(mode)) return
		if (mode === AIMode.SCRIPT && !tryGetCurrentModel()) return
		this.mode = mode
		this.pendingPrompt = pendingPrompt ?? ''
		if (mode === AIMode.SCRIPT) {
			const currentModel = getCurrentModel()
			const customPrompt = getCombinedCustomPrompt(mode)
			const lang = options?.lang ?? this.scriptEditorOptions?.lang ?? 'bun'
			const workflowAsCode =
				options?.workflowAsCode ??
				(options?.lang ? false : (this.scriptEditorOptions?.workflowAsCode ?? false))
			const context = this.contextManager.getSelectedContext()
			this.systemMessage = prepareScriptSystemMessage(
				currentModel,
				lang,
				{ isPreprocessor: options?.isPreprocessor, workflowAsCode },
				customPrompt
			)
			this.systemMessage.content = this.systemMessage.content
			this.tools = [...prepareScriptTools(currentModel, lang, context)]
			this.helpers = {
				getScriptOptions: () => {
					return {
						code: this.scriptEditorOptions?.code ?? '',
						lang: lang,
						path: this.scriptEditorOptions?.path ?? '',
						args: this.scriptEditorOptions?.args ?? {}
					}
				},
				getWorkspaceMutationTarget: this.getScriptWorkspaceMutationTarget,
				applyCode: (code: string, opts?: ReviewChangesOpts) => {
					return this.applyScriptEditorCode(code, opts)
				},
				getLintErrors: () => {
					if (this.scriptEditorGetLintErrors) {
						return this.scriptEditorGetLintErrors()
					}
					return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
				}
			}
			if (options?.closeScriptSettings) {
				const closeComponent = triggerablesByAi['close-script-builder-settings']
				if (closeComponent) {
					closeComponent.onTrigger?.()
				}
			}
		} else if (mode === AIMode.FLOW) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareFlowSystemMessage(customPrompt)
			this.systemMessage.content = this.systemMessage.content
			this.tools = [...flowTools]
			this.helpers = {
				...(this.flowAiChatHelpers ?? {}),
				getWorkspaceMutationTarget: this.getFlowWorkspaceMutationTarget
			}
		} else if (mode === AIMode.NAVIGATOR) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareNavigatorSystemMessage(customPrompt)
			this.tools = [this.changeModeTool, ...navigatorTools]
			this.helpers = {}
		} else if (mode === AIMode.ASK) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareAskSystemMessage(customPrompt)
			this.tools = [...askTools]
			this.helpers = {}
		} else if (mode === AIMode.API) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareApiSystemMessage(customPrompt)
			this.tools = [...this.apiTools]
			this.helpers = {}
		} else if (mode === AIMode.GLOBAL) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareGlobalSystemMessage(customPrompt)
			this.tools = [...globalTools]
			this.helpers = {}
		} else if (mode === AIMode.APP) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareAppSystemMessage(customPrompt)
			this.tools = [...getAppTools()]
			this.helpers = this.appAiChatHelpers
		}
	}

	canApplyCode = $derived(this.allowedModes.script && this.mode === AIMode.SCRIPT)

	private changeModeTool = {
		def: {
			type: 'function' as const,
			function: {
				name: 'change_mode',
				description:
					'Change the AI mode to the one specified. Script mode is used to create scripts. Flow mode is used to create flows.' +
					(isGlobalAiEnabled()
						? ' Global mode is used to inspect workspace scripts and flows and create draft changes.'
						: '') +
					' Navigator mode is used to navigate the application and help the user find what they are looking for. API mode is used to make API calls to the Windmill backend.',
				parameters: {
					type: 'object',
					properties: {
						mode: {
							type: 'string',
							description: 'The mode to change to',
							enum: [
								'script',
								'flow',
								...(isGlobalAiEnabled() ? ['global'] : []),
								'navigator',
								'API'
							]
						},
						pendingPrompt: {
							type: 'string',
							description: 'The prompt to send to the new mode to fulfill the user request',
							default: ''
						}
					},
					required: ['mode']
				}
			}
		},
		fn: async ({ args, toolId, toolCallbacks }) => {
			if (!isAIMode(args.mode) || !isAIModeVisible(args.mode)) {
				throw new Error(`AI mode "${args.mode}" is not enabled`)
			}
			toolCallbacks.setToolStatus(toolId, { content: 'Switching to ' + args.mode + ' mode...' })
			this.changeMode(args.mode, args.pendingPrompt, {
				closeScriptSettings: true
			})
			toolCallbacks.setToolStatus(toolId, { content: 'Switched to ' + args.mode + ' mode' })
			return 'Mode changed to ' + args.mode
		}
	}

	openChat = () => {
		chatState.size = this.savedSize > 0 ? this.savedSize : DEFAULT_SIZE
		localStorage.setItem('ai-chat-open', 'true')
	}

	closeChat = () => {
		this.savedSize = chatState.size
		chatState.size = 0
		localStorage.setItem('ai-chat-open', 'false')
	}

	toggleOpen = () => {
		if (chatState.size > 0) {
			this.savedSize = chatState.size
		}
		chatState.size = chatState.size === 0 ? (this.savedSize > 0 ? this.savedSize : DEFAULT_SIZE) : 0
		localStorage.setItem('ai-chat-open', chatState.size === 0 ? 'false' : 'true')
	}

	askAi = (
		prompt: string,
		options: { withCode?: boolean; withDiff?: boolean } = {
			withCode: true,
			withDiff: false
		}
	) => {
		if (this.scriptEditorOptions) {
			this.contextManager.setAskAiContext(options)
		}
		this.instructions = prompt
		this.sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			this.scriptEditorShowDiffMode?.()
		}
	}

	retryRequest = (messageIndex: number) => {
		const message = this.displayMessages[messageIndex]
		if (message && message.role === 'user') {
			this.restartGeneration(messageIndex)
			message.error = false
		} else {
			throw new Error('No user message found at the specified index')
		}
	}

	private getLastUserMessage = () => {
		for (let i = this.displayMessages.length - 1; i >= 0; i--) {
			const message = this.displayMessages[i]
			if (message.role === 'user') {
				return message
			}
		}
	}

	private flagLastMessageAsError = () => {
		const lastUserMessage = this.getLastUserMessage()
		if (lastUserMessage) {
			lastUserMessage.error = true
		}
	}

	private chatRequest = async ({
		messages,
		abortController,
		callbacks,
		systemMessage: systemMessageOverride
	}: {
		messages: ChatCompletionMessageParam[]
		abortController: AbortController
		callbacks: ToolCallbacks & {
			onNewToken: (token: string) => void
			onMessageEnd: () => void
		}
		systemMessage?: ChatCompletionSystemMessageParam
	}) => {
		try {
			// Use JS getters so runChatLoop re-reads tools/helpers/systemMessage/modelProvider
			// on each iteration. This is critical for changeModeTool (Navigator → Script/Flow)
			// which reassigns this.tools, this.helpers, this.systemMessage mid-loop.
			const self = this
			const result = await runChatLoop({
				messages,
				get systemMessage() {
					return systemMessageOverride ?? self.systemMessage
				},
				get tools() {
					return self.tools
				},
				get helpers() {
					return self.helpers
				},
				abortController,
				callbacks,
				get modelProvider() {
					return getCurrentModel()
				},
				clients: {
					openai: workspaceAIClients.getOpenaiClient(),
					anthropic: workspaceAIClients.getAnthropicClient()
				},
				workspace: get(workspaceStore) ?? '',
				skipResponsesApi: this.skipResponsesApi,
				onSkipResponsesApi: () => {
					this.skipResponsesApi = true
				},
				getPendingUserMessage: () => {
					const pendingPrompt = this.pendingPrompt
					if (!pendingPrompt) return undefined
					this.pendingPrompt = ''
					if (this.mode === AIMode.SCRIPT) {
						return prepareScriptUserMessage(pendingPrompt, this.contextManager.getSelectedContext())
					} else if (this.mode === AIMode.FLOW) {
						return prepareFlowUserMessage(
							pendingPrompt,
							this.flowAiChatHelpers!.getFlowAndSelectedId(),
							[],
							this.flowAiChatHelpers!.inlineScriptSession
						)
					} else if (this.mode === AIMode.NAVIGATOR) {
						return prepareNavigatorUserMessage(pendingPrompt)
					} else if (this.mode === AIMode.GLOBAL) {
						return prepareGlobalUserMessage(
							pendingPrompt,
							this.contextManager.getSelectedContext(),
							{ workspace: get(workspaceStore) }
						)
					}
					return undefined
				},
				onBeforeIteration: async (tools) => {
					for (const tool of tools) {
						if (tool.setSchema) {
							await tool.setSchema(this.helpers)
						}
					}
				}
			})
			return result.addedMessages
		} catch (err) {
			console.log('chatRequest error', err)
			console.error('chatRequest error', err)
			callbacks.onMessageEnd()
			this.cancelLoadingTools('Error')
			if (!abortController.signal.aborted) {
				throw err
			}
		}
	}

	sendInlineRequest = async (instructions: string, selectedCode: string, selection: Selection) => {
		// Validate inputs
		if (!instructions.trim()) {
			throw new Error('Instructions are required')
		}
		// Use a separate abort controller for inline requests to avoid interfering with main chat
		this.inlineAbortController = new AbortController()
		const lang = this.scriptEditorOptions?.lang ?? 'bun'
		const selectedContext: ContextElement[] = [...this.contextManager.getSelectedContext()]
		const startLine = selection.startLineNumber
		const endLine = selection.endLineNumber
		selectedContext.push({
			type: 'code_piece',
			lang,
			title: `L${startLine}-L${endLine}`,
			startLine,
			endLine,
			content: selectedCode
		})

		const systemMessage: ChatCompletionSystemMessageParam = {
			role: 'system',
			content: prepareInlineChatSystemPrompt(lang, {
				workflowAsCode: this.scriptEditorOptions?.workflowAsCode ?? false
			})
		}

		let reply = ''

		try {
			const userMessage = prepareScriptUserMessage(instructions, selectedContext)
			const messages = [userMessage]

			const params = {
				messages,
				abortController: this.inlineAbortController,
				callbacks: {
					onNewToken: (token: string) => {
						reply += token
					},
					onMessageEnd: () => {},
					setToolStatus: () => {},
					removeToolStatus: () => {}
				},
				systemMessage
			}

			await this.chatRequest({ ...params })

			// Validate we received a response
			if (!reply.trim()) {
				throw new Error('AI response was empty')
			}

			// Try to extract new code from response
			const newCodeMatch = reply.match(/<new_code>([\s\S]*?)<\/new_code>/i)
			if (newCodeMatch && newCodeMatch[1]) {
				const code = newCodeMatch[1].trim()
				if (!code) {
					throw new Error('AI response contained empty code block')
				}
				return code
			}

			// Fallback: try to take everything after the last <new_code> tag
			const lastNewCodeMatch = reply.match(/<new_code>([\s\S]*)/i)
			if (lastNewCodeMatch && lastNewCodeMatch[1]) {
				const code = lastNewCodeMatch[1].trim().replace(/```/g, '')
				if (!code) {
					throw new Error('AI response contained empty code block')
				}
				return code
			}

			// If no code tags found, throw error with helpful message
			throw new Error('AI response did not contain valid code. Please try rephrasing your request.')
		} catch (error) {
			// if abort controller is aborted, don't throw an error
			if (this.inlineAbortController?.signal.aborted) {
				return
			}
			console.error('Unexpected error in sendInlineRequest:', error)
			throw new Error('An unexpected error occurred. Please try again.')
		}
	}

	sendRequest = async (
		options: {
			removeDiff?: boolean
			addBackCode?: boolean
			instructions?: string
			mode?: AIMode
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
		} = {}
	) => {
		const requestedMode = options.mode ?? this.mode
		if (!isAIModeVisible(requestedMode)) {
			return
		}
		this.changeMode(requestedMode, undefined, {
			lang: options.lang,
			isPreprocessor: options.isPreprocessor
		})
		if (options.instructions) {
			this.instructions = options.instructions
		}
		if (!this.instructions.trim()) {
			return
		}
		try {
			const oldSelectedContext = this.contextManager?.getSelectedContext() ?? []
			if (this.mode === AIMode.SCRIPT || this.mode === AIMode.FLOW) {
				this.contextManager?.updateContextOnRequest(options)
			}
			this.loading = true
			this.#automaticScroll = true
			this.abortController = new AbortController()

			const model = tryGetCurrentModel()
			if (model) {
				WorkspaceService.logAiChat({
					workspace: get(workspaceStore) ?? '',
					requestBody: {
						session_id: this.historyManager.getCurrentChatId(),
						provider: model.provider,
						model: model.model,
						mode: this.mode
					}
				}).catch(() => {})
			}

			if (this.mode === AIMode.FLOW && !this.flowAiChatHelpers) {
				throw new Error('No flow helpers found')
			}

			let snapshot:
				| { type: 'flow'; value: ExtendedOpenFlow }
				| { type: 'app'; value: number }
				| undefined = undefined
			if (this.mode === AIMode.FLOW) {
				snapshot = { type: 'flow', value: this.flowAiChatHelpers!.getFlowAndSelectedId().flow }
				this.flowAiChatHelpers!.setSnapshot(snapshot.value)
			} else if (this.mode === AIMode.APP) {
				snapshot = { type: 'app', value: this.appAiChatHelpers!.snapshot() }
			}

			this.displayMessages = [
				...this.displayMessages,
				{
					role: 'user',
					content: this.instructions,
					contextElements:
						this.mode === AIMode.SCRIPT || this.mode === AIMode.FLOW || this.mode === AIMode.GLOBAL
							? oldSelectedContext
							: undefined,
					snapshot,
					index: this.messages.length // matching with actual messages index. not -1 because it's not yet added to the messages array
				}
			]
			const oldInstructions = this.instructions
			this.instructions = ''

			if (this.mode === AIMode.SCRIPT && !this.scriptEditorOptions && !options.lang) {
				throw new Error('No script options passed')
			}

			let userMessage: ChatCompletionMessageParam = {
				role: 'user',
				content: ''
			}
			switch (this.mode) {
				case AIMode.FLOW:
					userMessage = prepareFlowUserMessage(
						oldInstructions,
						this.flowAiChatHelpers!.getFlowAndSelectedId(),
						oldSelectedContext,
						this.flowAiChatHelpers!.inlineScriptSession
					)
					break
				case AIMode.NAVIGATOR:
					userMessage = prepareNavigatorUserMessage(oldInstructions)
					break
				case AIMode.ASK:
					userMessage = prepareAskUserMessage(oldInstructions)
					break
				case AIMode.SCRIPT:
					userMessage = prepareScriptUserMessage(oldInstructions, oldSelectedContext)
					break
				case AIMode.API:
					userMessage = prepareApiUserMessage(oldInstructions)
					break
				case AIMode.GLOBAL:
					userMessage = prepareGlobalUserMessage(oldInstructions, oldSelectedContext, {
						workspace: get(workspaceStore)
					})
					break
				case AIMode.APP:
					userMessage = prepareAppUserMessage(
						oldInstructions,
						this.appAiChatHelpers?.getSelectedContext(),
						oldSelectedContext
					)
					break
			}

			this.messages.push(userMessage)
			await this.historyManager.saveChat(this.displayMessages, this.messages)

			this.currentReply = ''

			let trimmedMessages = [...this.messages]
			if (this.checkTokenUsageOverLimit(trimmedMessages)) {
				trimmedMessages = this.deleteOldestMessage(trimmedMessages)
			}

			const params: {
				messages: ChatCompletionMessageParam[]
				abortController: AbortController
				callbacks: ToolCallbacks & {
					onNewToken: (token: string) => void
					onMessageEnd: () => void
				}
			} = {
				messages: trimmedMessages,
				abortController: this.abortController,
				callbacks: {
					onNewToken: (token) => (this.currentReply += token),
					onMessageEnd: () => {
						if (this.currentReply) {
							this.displayMessages = [
								...this.displayMessages,
								{
									role: 'assistant',
									content: this.currentReply,
									contextElements:
										this.mode === AIMode.SCRIPT
											? oldSelectedContext.filter((c) => c.type === 'code')
											: undefined
								}
							]
						}
						this.currentReply = ''
					},
					setToolStatus: (id, metadata) => {
						const existingIdx = this.displayMessages.findIndex(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingIdx !== -1) {
							// Update existing tool message with metadata
							const existing = this.displayMessages[existingIdx] as ToolDisplayMessage
							if (existing.content.length === 0 && metadata?.error) {
								this.displayMessages[existingIdx].content = metadata.error
							}
							this.displayMessages[existingIdx] = {
								...existing,
								...(metadata || {})
							} as ToolDisplayMessage
						} else {
							// Create new tool message with metadata
							const newMessage: ToolDisplayMessage = {
								role: 'tool',
								tool_call_id: id,
								content: metadata?.content ?? metadata?.error ?? '',
								...(metadata || {})
							}
							this.displayMessages.push(newMessage)
						}
					},
					removeToolStatus: (id) => {
						const existingIdx = this.displayMessages.findIndex(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingIdx !== -1) {
							this.displayMessages.splice(existingIdx, 1)
							this.displayMessages = [...this.displayMessages]
						}
					},
					requestConfirmation: this.requestConfirmation,
					shouldAutoAcceptToolConfirmations: () => this.autoAcceptToolConfirmationsActive,
					requestUserQuestion: this.requestUserQuestion
				}
			}

			if (this.mode === AIMode.API && this.apiTools.length === 0) {
				await this.loadApiTools()
			}

			const addedMessages = await this.chatRequest({
				...params
			})
			this.messages = [...this.messages, ...(addedMessages ?? [])]
			if (this.autoAcceptEditsActive) {
				this.acceptPendingFlowEdits()
			}
			await this.historyManager.saveChat(this.displayMessages, this.messages)
		} catch (err) {
			console.error(err)
			this.flagLastMessageAsError()
			if (err instanceof Error) {
				sendUserToast('Failed to send request: ' + err.message, true)
			} else {
				sendUserToast('Failed to send request', true)
			}
		} finally {
			this.loading = false
		}
	}

	cancel = (reason?: string) => {
		for (const confirmationCallback of this.confirmationCallbacks.values()) {
			confirmationCallback(false)
		}
		this.confirmationCallbacks.clear()
		for (const resolveQuestion of this.userQuestionCallbacks.values()) {
			resolveQuestion(undefined)
		}
		this.userQuestionCallbacks.clear()
		const cancelReason = reason ?? 'user_cancelled'
		console.log('cancelling request:', {
			reason: cancelReason,
			abortController: this.abortController
		})
		this.abortController?.abort(cancelReason)
		this.cancelLoadingTools()
	}

	cancelInlineRequest = (reason?: string) => {
		const cancelReason = reason ?? 'inline_cancelled'
		console.log('cancelling inline request:', {
			reason: cancelReason,
			inlineAbortController: this.inlineAbortController
		})
		this.inlineAbortController?.abort(cancelReason)
	}

	restartGeneration = (displayMessageIndex: number, newContent?: string) => {
		const userMessage = this.displayMessages[displayMessageIndex]

		if (!userMessage || userMessage.role !== 'user') {
			throw new Error('No user message found at the specified index')
		}

		// Remove all messages including and after the specified user message
		this.displayMessages = this.displayMessages.slice(0, displayMessageIndex)

		// Find corresponding message in actual messages and remove it and everything after it
		let actualMessageIndex = this.messages.findIndex((_, i) => i === userMessage.index)

		if (actualMessageIndex === -1) {
			throw new Error('No actual user message found to restart from')
		}

		this.messages = this.messages.slice(0, actualMessageIndex)

		// Resend the request with the same instructions
		this.instructions = newContent ?? userMessage.content
		this.sendRequest()
	}

	fix = () => {
		if (!this.open) {
			this.toggleOpen()
		}
		this.changeMode(AIMode.SCRIPT)
		this.instructions = 'Fix the error'
		this.contextManager?.setFixContext()
		this.sendRequest()
	}

	addSelectedLinesToContext = (
		lines: string,
		startLine: number,
		endLine: number,
		moduleId?: string
	) => {
		if (!this.open) {
			this.toggleOpen()
		}
		if (!moduleId) {
			this.changeMode(AIMode.SCRIPT)
		}
		this.contextManager?.addSelectedLinesToContext(lines, startLine, endLine, moduleId)
		this.focusInput()
	}

	saveAndClear = async () => {
		this.cancel('saveAndClear')
		await this.historyManager.save(this.displayMessages, this.messages)
		this.displayMessages = []
		this.messages = []
	}

	loadPastChat = async (id: string) => {
		const chat = this.historyManager.loadPastChat(id)
		if (chat) {
			this.displayMessages = chat.displayMessages
			this.messages = chat.actualMessages
			this.#automaticScroll = true
		}
	}

	get automaticScroll() {
		return this.#automaticScroll
	}

	disableAutomaticScroll = () => {
		this.#automaticScroll = false
	}

	enableAutomaticScroll = () => {
		this.#automaticScroll = true
	}

	generateStep = async (moduleId: string, lang: ScriptLang, instructions: string) => {
		if (!this.flowAiChatHelpers) {
			throw new Error('No flow helpers found')
		}
		this.flowAiChatHelpers.selectStep(moduleId)
		await this.sendRequest({
			instructions: instructions,
			mode: AIMode.SCRIPT,
			lang: lang,
			isPreprocessor: moduleId === 'preprocessor'
		})
	}

	listenForContextChange = (
		dbSchemas: DBSchemas,
		workspaceStore: string | undefined,
		copilotSessionModel: AIProviderModel | undefined
	) => {
		if (this.mode === AIMode.SCRIPT && this.scriptEditorOptions) {
			this.contextManager.updateAvailableContext(
				this.scriptEditorOptions,
				dbSchemas,
				workspaceStore ?? '',
				!copilotSessionModel?.model.endsWith('/thinking'),
				untrack(() => this.contextManager.getSelectedContext())
			)
		} else if (this.mode === AIMode.FLOW && this.flowOptions) {
			this.contextManager.updateAvailableContextForFlow(
				this.flowOptions,
				dbSchemas,
				workspaceStore ?? '',
				!copilotSessionModel?.model.endsWith('/thinking'),
				untrack(() => this.contextManager.getSelectedContext())
			)
		} else if (this.mode === AIMode.GLOBAL) {
			this.contextManager.updateAvailableContextForGlobal(
				workspaceStore ?? '',
				untrack(() => this.contextManager.getSelectedContext())
			)
		}

		if (this.scriptEditorOptions) {
			this.contextManager.setScriptOptions(this.scriptEditorOptions)
		}
	}

	listenForDbSchemasChanges = (dbSchemas: DBSchemas) => {
		this.displayMessages = ContextManager.updateDisplayMessages(
			untrack(() => this.displayMessages),
			dbSchemas
		)
	}

	listenForCurrentEditorChanges = (currentEditor: CurrentEditor) => {
		if (currentEditor && currentEditor.type === 'script') {
			this.scriptEditorApplyCode = async (code, opts) => {
				if (currentEditor && currentEditor.type === 'script') {
					currentEditor.hideDiffMode()
					await currentEditor.editor.reviewAndApplyCode(code, opts)
				}
			}
			this.scriptEditorShowDiffMode = () => {
				if (currentEditor && currentEditor.type === 'script') {
					currentEditor.showDiffMode()
				}
			}
			this.scriptEditorGetLintErrors = () => {
				if (currentEditor && currentEditor.type === 'script') {
					return currentEditor.editor.getLintErrors()
				}
				return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
			}
		} else {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
			this.scriptEditorGetLintErrors = undefined
		}

		return () => {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
			this.scriptEditorGetLintErrors = undefined
		}
	}

	listenForSelectedIdChanges = (
		selectedId: string | undefined,
		flowStore: ExtendedOpenFlow,
		flowStateStore: FlowState,
		currentEditor: CurrentEditor
	) => {
		function getModule(id: string) {
			if (id === 'preprocessor') {
				return flowStore.value.preprocessor_module
			} else if (id === 'failure') {
				return flowStore.value.failure_module
			} else {
				return dfs(id, flowStore, false)[0]
			}
		}

		function getScriptOptions(id: string): ScriptOptions | undefined {
			const module = getModule(id)

			if (module && module.value.type === 'rawscript') {
				const moduleState: FlowModuleState | undefined = flowStateStore[module.id]

				const editorRelated =
					currentEditor && currentEditor.type === 'script' && currentEditor.stepId === module.id
						? {
								diffMode: currentEditor.diffMode,
								lastDeployedCode: currentEditor.lastDeployedCode,
								lastSavedCode: undefined
							}
						: {
								diffMode: false,
								lastDeployedCode: undefined,
								lastSavedCode: undefined
							}

				return {
					args: moduleState?.previewArgs ?? {},
					error:
						moduleState && !moduleState.previewSuccess
							? getStringError(moduleState.previewResult)
							: undefined,
					code: module.value.content,
					lang: module.value.language,
					path: module.id,
					...editorRelated
				}
			}

			return undefined
		}

		if (selectedId) {
			const options = getScriptOptions(selectedId)
			if (options) {
				this.scriptEditorOptions = options
			}
		} else {
			this.scriptEditorOptions = undefined
		}

		untrack(() =>
			this.contextManager?.setSelectedModuleContext(
				selectedId,
				untrack(() => this.contextManager.getAvailableContext())
			)
		)

		return () => {
			this.scriptEditorOptions = undefined
		}
	}

	setFlowHelpers = (flowHelpers: FlowAIChatHelpers) => {
		this.flowAiChatHelpers = flowHelpers
		untrack(() => {
			if (this.autoAcceptEditsActive) {
				this.acceptPendingFlowEdits(flowHelpers)
			}
		})

		return () => {
			this.flowAiChatHelpers = undefined
		}
	}

	/**
	 * Refresh cached datatables from the app helpers (async)
	 * Creates one context element per table (not per datatable)
	 */
	refreshDatatables = async (): Promise<void> => {
		if (!this.appAiChatHelpers) {
			this.cachedDatatables = []
			return
		}

		try {
			const datatables = await this.appAiChatHelpers.listDatatableTables()
			this.cachedDatatables = flattenDatatablesToAppContextElements(datatables)
		} catch (err) {
			console.error('Failed to refresh datatables:', err)
			this.cachedDatatables = []
		}
	}

	/**
	 * Get available context elements for app mode (frontend files + backend runnables + datatables)
	 */
	getAppAvailableContext = (): ContextElement[] => {
		if (!this.appAiChatHelpers) {
			return []
		}

		const context: ContextElement[] = []

		// Add frontend files
		const frontendFiles = this.appAiChatHelpers.listFrontendFiles()
		for (const path of frontendFiles) {
			const content = this.appAiChatHelpers.getFrontendFile(path)
			if (content !== undefined) {
				context.push(createAppFrontendFileContextElement(path, content))
			}
		}

		// Add backend runnables
		const runnables = this.appAiChatHelpers.listBackendRunnables()
		for (const { key } of runnables) {
			const runnable = this.appAiChatHelpers.getBackendRunnable(key)
			if (runnable) {
				context.push(createAppBackendRunnableContextElement(key, runnable))
			}
		}

		// Add cached datatables
		context.push(...this.cachedDatatables)

		return context
	}

	setAppHelpers = (appHelpers: AppAIChatHelpers) => {
		this.appAiChatHelpers = appHelpers
		// Refresh datatables when app helpers are set (deferred to avoid loop)
		// Use setTimeout to ensure this runs after the effect completes
		if (this.appDatatablesRefreshTimeout) {
			clearTimeout(this.appDatatablesRefreshTimeout)
		}
		this.appDatatablesRefreshTimeout = setTimeout(() => {
			this.appDatatablesRefreshTimeout = undefined
			if (this.appAiChatHelpers === appHelpers) {
				void this.refreshDatatables()
			}
		}, 50)

		return () => {
			if (this.appDatatablesRefreshTimeout) {
				clearTimeout(this.appDatatablesRefreshTimeout)
				this.appDatatablesRefreshTimeout = undefined
			}
			if (this.appAiChatHelpers === appHelpers) {
				this.appAiChatHelpers = undefined
				this.cachedDatatables = []
			}
		}
	}

	cancelLoadingTools = (messageText: 'Canceled' | 'Error' = 'Canceled') => {
		this.displayMessages = this.displayMessages.map((message) => {
			if (message.role === 'tool' && message.isLoading) {
				return {
					...message,
					isLoading: false,
					content: messageText,
					error: messageText,
					userQuestion: message.userQuestion
						? { ...message.userQuestion, canceled: true }
						: undefined
				}
			}
			return message
		})
	}
}

export const aiChatManager = new AIChatManager()
