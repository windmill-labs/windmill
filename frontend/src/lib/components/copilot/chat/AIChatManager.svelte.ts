import type { AIProviderModel, ScriptLang } from '$lib/gen/types.gen'
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
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
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
import { getCompletion, getModelContextWindow, parseOpenAICompletion } from '../lib'
import { dfs } from '$lib/components/flows/previousResults'
import { getStringError } from './utils'
import type { FlowModuleState, FlowState } from '$lib/components/flows/flowState'
import type { CurrentEditor, ExtendedOpenFlow } from '$lib/components/flows/types'
import { untrack } from 'svelte'
import { type DBSchemas } from '$lib/stores'
import { askTools, prepareAskSystemMessage, prepareAskUserMessage } from './ask/core'
import { chatState, DEFAULT_SIZE, triggerablesByAi } from './sharedChatState.svelte'
import type {
	ContextElement,
	AppFrontendFileElement,
	AppBackendRunnableElement,
	AppDatatableElement
} from './context'
import type { Selection } from 'monaco-editor'
import type AIChatInput from './AIChatInput.svelte'
import { prepareApiSystemMessage, prepareApiUserMessage } from './api/core'
import { getAnthropicCompletion, parseAnthropicCompletion } from './anthropic'
import { getOpenAIResponsesCompletion, parseOpenAIResponsesCompletion } from './openai-responses'
import type { ReviewChangesOpts } from './monaco-adapter'
import { getCurrentModel, tryGetCurrentModel, getCombinedCustomPrompt } from '$lib/aiStore'

// If the estimated token usage is greater than the model context window - the threshold, we delete the oldest message
const MAX_TOKENS_THRESHOLD_PERCENTAGE = 0.05
const MAX_TOKENS_HARD_LIMIT = 5000

export enum AIMode {
	SCRIPT = 'script',
	FLOW = 'flow',
	APP = 'app',
	NAVIGATOR = 'navigator',
	API = 'API',
	ASK = 'ask'
}

class AIChatManager {
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
	#automaticScroll = $state<boolean>(true)
	systemMessage = $state<ChatCompletionSystemMessageParam>({
		role: 'system',
		content: ''
	})
	tools = $state<Tool<any>[]>([])
	helpers = $state<any | undefined>(undefined)

	scriptEditorOptions = $state<ScriptOptions | undefined>(undefined)
	flowOptions = $state<FlowOptions | undefined>(undefined)
	scriptEditorApplyCode = $state<((code: string, opts?: ReviewChangesOpts) => void) | undefined>(
		undefined
	)
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

	private confirmationCallback = $state<((value: boolean) => void) | undefined>(undefined)

	allowedModes: Record<AIMode, boolean> = $derived({
		script: this.flowAiChatHelpers === undefined && this.scriptEditorOptions !== undefined,
		flow: this.flowAiChatHelpers !== undefined,
		app: this.appAiChatHelpers !== undefined,
		navigator: true,
		ask: true,
		API: true
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
		return new Promise((resolve) => {
			// Store the callback for this specific tool
			this.confirmationCallback = resolve
		})
	}

	// Handle confirmation response for a specific tool
	handleToolConfirmation = (toolId: string, confirmed: boolean) => {
		if (this.confirmationCallback) {
			this.confirmationCallback(confirmed)
			this.confirmationCallback = undefined
		}
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

	changeMode(
		mode: AIMode,
		pendingPrompt?: string,
		options?: {
			closeScriptSettings?: boolean
		}
	) {
		if (mode === AIMode.SCRIPT && !tryGetCurrentModel()) return
		this.mode = mode
		this.pendingPrompt = pendingPrompt ?? ''
		if (mode === AIMode.SCRIPT) {
			const currentModel = getCurrentModel()
			const customPrompt = getCombinedCustomPrompt(mode)
			const lang = this.scriptEditorOptions?.lang ?? 'bun'
			const context = this.contextManager.getSelectedContext()
			this.systemMessage = prepareScriptSystemMessage(currentModel, lang, {}, customPrompt)
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
				applyCode: (code: string, opts?: ReviewChangesOpts) => {
					this.scriptEditorApplyCode?.(code, opts)
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
			this.helpers = this.flowAiChatHelpers
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
					'Change the AI mode to the one specified. Script mode is used to create scripts. Flow mode is used to create flows. Navigator mode is used to navigate the application and help the user find what they are looking for. API mode is used to make API calls to the Windmill backend.',
				parameters: {
					type: 'object',
					properties: {
						mode: {
							type: 'string',
							description: 'The mode to change to',
							enum: ['script', 'flow', 'navigator', 'API']
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
			toolCallbacks.setToolStatus(toolId, { content: 'Switching to ' + args.mode + ' mode...' })
			this.changeMode(args.mode as AIMode, args.pendingPrompt, {
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
			let addedMessages: ChatCompletionMessageParam[] = []
			while (true) {
				const systemMessage = systemMessageOverride ?? this.systemMessage
				const helpers = this.helpers
				const tools = this.tools
				for (const tool of tools) {
					if (tool.setSchema) {
						await tool.setSchema(helpers)
					}
				}

				let pendingPrompt = this.pendingPrompt
				let pendingUserMessage: ChatCompletionUserMessageParam | undefined = undefined
				if (pendingPrompt) {
					if (this.mode === AIMode.SCRIPT) {
						pendingUserMessage = prepareScriptUserMessage(
							pendingPrompt,
							this.contextManager.getSelectedContext()
						)
					} else if (this.mode === AIMode.FLOW) {
						pendingUserMessage = prepareFlowUserMessage(
							pendingPrompt,
							this.flowAiChatHelpers!.getFlowAndSelectedId()
						)
					} else if (this.mode === AIMode.NAVIGATOR) {
						pendingUserMessage = prepareNavigatorUserMessage(pendingPrompt)
					}
					this.pendingPrompt = ''
				}

				const model = getCurrentModel()
				const isOpenAI = model.provider === 'openai' || model.provider === 'azure_openai'
				const isAnthropic = model.provider === 'anthropic'

				const messageParams = [
					systemMessage,
					...messages,
					...(pendingUserMessage ? [pendingUserMessage] : [])
				]
				const toolDefs = tools.map((t) => t.def)

				// For OpenAI/Azure, try Responses API first, fallback to Completions API
				if (isOpenAI) {
					let useCompletionsApi = this.skipResponsesApi
					if (!this.skipResponsesApi) {
						try {
							const completion = await getOpenAIResponsesCompletion(
								messageParams,
								abortController,
								toolDefs
							)
							const continueCompletion = await parseOpenAIResponsesCompletion(
								completion,
								callbacks,
								messages,
								addedMessages,
								tools,
								helpers
							)
							if (!continueCompletion) {
								break
							}
						} catch (err) {
							console.warn('OpenAI Responses API failed, falling back to Completions API:', err)
							// If the error indicates Responses API is not available in this region, skip it for future requests
							const errorMessage = err instanceof Error ? err.message : String(err)
							if (errorMessage.includes('Responses API is not enabled')) {
								this.skipResponsesApi = true
							}
							useCompletionsApi = true
						}
					}

					// Use Completions API if Responses API is not available or failed
					if (useCompletionsApi) {
						const completion = await getCompletion(messageParams, abortController, toolDefs, {
							forceCompletions: true
						})
						const continueCompletion = await parseOpenAICompletion(
							completion,
							callbacks,
							messages,
							addedMessages,
							tools,
							helpers
						)
						if (!continueCompletion) {
							break
						}
					}
				} else if (isAnthropic) {
					const completion = await getAnthropicCompletion(messageParams, abortController, toolDefs)
					if (completion) {
						const continueCompletion = await parseAnthropicCompletion(
							completion,
							callbacks,
							messages,
							addedMessages,
							tools,
							helpers,
							abortController
						)
						if (!continueCompletion) {
							break
						}
					}
				} else {
					const completion = await getCompletion(messageParams, abortController, toolDefs)
					if (completion) {
						const continueCompletion = await parseOpenAICompletion(
							completion,
							callbacks,
							messages,
							addedMessages,
							tools,
							helpers
						)
						if (!continueCompletion) {
							break
						}
					}
				}
			}
			return addedMessages
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
			content: prepareInlineChatSystemPrompt(lang)
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
		if (options.mode) {
			this.changeMode(options.mode)
		} else {
			this.changeMode(this.mode)
		}
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
						this.mode === AIMode.SCRIPT || this.mode === AIMode.FLOW
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
						oldSelectedContext
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
					requestConfirmation: this.requestConfirmation
				}
			}

			if (this.mode === AIMode.API && this.apiTools.length === 0) {
				await this.loadApiTools()
			}

			const addedMessages = await this.chatRequest({
				...params
			})
			this.messages = [...this.messages, ...(addedMessages ?? [])]
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
		if (this.confirmationCallback) {
			this.confirmationCallback(false)
			this.confirmationCallback = undefined
		}
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
			this.scriptEditorApplyCode = (code) => {
				if (currentEditor && currentEditor.type === 'script') {
					currentEditor.hideDiffMode()
					currentEditor.editor.reviewAndApplyCode(code)
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
			const datatables = await this.appAiChatHelpers.getDatatables()
			console.log('Refreshed datatables:', datatables)

			// Flatten to individual tables
			const tableElements: AppDatatableElement[] = []
			for (const dt of datatables) {
				if (dt.error) {
					// Skip datatables with errors
					continue
				}
				for (const [schemaName, tables] of Object.entries(dt.schemas)) {
					for (const [tableName, columns] of Object.entries(tables)) {
						// Format title as "datatable/schema:table" or "datatable/table" if schema is public
						const title =
							schemaName === 'public'
								? `${dt.datatable_name}/${tableName}`
								: `${dt.datatable_name}/${schemaName}:${tableName}`
						tableElements.push({
							type: 'app_datatable',
							datatableName: dt.datatable_name,
							schemaName,
							tableName,
							title,
							columns
						})
					}
				}
			}
			this.cachedDatatables = tableElements
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
				const element: AppFrontendFileElement = {
					type: 'app_frontend_file',
					path,
					title: path,
					content
				}
				context.push(element)
			}
		}

		// Add backend runnables
		const runnables = this.appAiChatHelpers.listBackendRunnables()
		for (const { key } of runnables) {
			const runnable = this.appAiChatHelpers.getBackendRunnable(key)
			if (runnable) {
				const element: AppBackendRunnableElement = {
					type: 'app_backend_runnable',
					key,
					title: key,
					runnable
				}
				context.push(element)
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
		setTimeout(() => {
			this.refreshDatatables()
		}, 50)

		return () => {
			this.appAiChatHelpers = undefined
			this.cachedDatatables = []
		}
	}

	cancelLoadingTools = (messageText: 'Canceled' | 'Error' = 'Canceled') => {
		this.displayMessages = this.displayMessages.map((message) => {
			if (message.role === 'tool' && message.isLoading) {
				return {
					...message,
					isLoading: false,
					content: messageText,
					error: messageText
				}
			}
			return message
		})
	}
}

export const aiChatManager = new AIChatManager()
