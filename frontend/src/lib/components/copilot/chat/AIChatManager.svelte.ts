import type { ScriptLang } from '$lib/gen/types.gen'
import type { ScriptOptions } from './ContextManager.svelte'
import {
	flowTools,
	prepareFlowSystemMessage,
	prepareFlowUserMessage,
	type FlowAIChatHelpers
} from './flow/core'
import ContextManager from './ContextManager.svelte'
import HistoryManager from './HistoryManager.svelte'
import { chatRequest, type DisplayMessage, type Tool, type ToolCallbacks } from './shared'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam
} from 'openai/resources/chat/completions.mjs'
import { prepareScriptSystemMessage, prepareScriptTools } from './script/core'
import { navigatorTools, prepareNavigatorSystemMessage } from './navigator/core'
import { prepareScriptUserMessage } from './script/core'
import { prepareNavigatorUserMessage } from './navigator/core'
import { sendUserToast } from '$lib/toast'

type TriggerablesMap = Record<
	string,
	{ description: string; onTrigger: ((id: string) => void) | undefined }
>

export class AIChat {
	SIZE = 300

	contextManager = new ContextManager()
	historyManager = new HistoryManager()
	abortController: AbortController | undefined = undefined

	open = $state<boolean>(false)
	instructions = $state<string>('')
	loading = $state<boolean>(false)
	currentReply = $state<string>('')
	displayMessages = $state<DisplayMessage[]>([])
	messages = $state<ChatCompletionMessageParam[]>([])
	automaticScroll = $state<boolean>(true)
	systemMessage = $state<ChatCompletionSystemMessageParam>({
		role: 'system',
		content: ''
	})
	tools = $state<Tool<any>[]>([])
	helpers = $state<any | undefined>(undefined)

	triggerablesByAI = $state<TriggerablesMap>({})
	scriptEditorOptions = $state<ScriptOptions | undefined>(undefined)
	scriptEditorApplyCode = $state<((code: string) => void) | undefined>(undefined)
	scriptEditorShowDiffMode = $state<(() => void) | undefined>(undefined)
	flowAiChatHelpers = $state<FlowAIChatHelpers | undefined>(undefined)
	mode = $state<'script' | 'flow' | 'navigator'>('navigator')

	allowedModes = $derived({
		script: this.scriptEditorOptions !== undefined,
		flow: this.flowAiChatHelpers !== undefined,
		navigator: true
	})

	updateMode(currentMode: 'script' | 'flow' | 'navigator') {
		if (
			!this.allowedModes[currentMode] &&
			Object.keys(this.allowedModes).filter((k) => this.allowedModes[k]).length === 1
		) {
			const firstKey = Object.keys(this.allowedModes).filter((k) => this.allowedModes[k])[0]
			this.changeMode(firstKey as 'script' | 'flow' | 'navigator')
		}
	}

	changeMode(mode: 'script' | 'flow' | 'navigator') {
		this.mode = mode
		if (mode === 'script') {
			this.systemMessage = prepareScriptSystemMessage()
			const context = this.contextManager.getSelectedContext()
			const lang = this.scriptEditorOptions?.lang ?? 'bun'
			this.tools = prepareScriptTools(lang, context)
			this.helpers = {
				getLang: () => lang
			}
		} else if (mode === 'flow') {
			this.systemMessage = prepareFlowSystemMessage()
			this.tools = flowTools
			this.helpers = this.flowAiChatHelpers
		} else if (mode === 'navigator') {
			this.systemMessage = prepareNavigatorSystemMessage()
			this.tools = navigatorTools
			this.helpers = {}
		}
	}

	canApplyCode = $derived(this.allowedModes.script && this.mode === 'script')

	// constructor() {
	// 	$effect(() => {
	// 		this.updateMode(untrack(() => this.mode))
	// 	})
	// }

	toggleOpen = () => {
		this.open = !this.open
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

	sendRequest = async (
		options: {
			removeDiff?: boolean
			addBackCode?: boolean
			instructions?: string
			mode?: 'script' | 'flow' | 'navigator'
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
			if (this.mode === 'script') {
				this.contextManager?.updateContextOnRequest(options)
			}
			this.loading = true
			this.automaticScroll = true
			this.abortController = new AbortController()

			this.displayMessages = [
				...this.displayMessages,
				{
					role: 'user',
					content: this.instructions,
					contextElements: this.mode === 'script' ? oldSelectedContext : undefined
				}
			]
			const oldInstructions = this.instructions
			this.instructions = ''

			if (this.mode === 'flow' && !this.flowAiChatHelpers) {
				throw new Error('No flow helpers passed')
			}

			if (this.mode === 'script' && !this.scriptEditorOptions && !options.lang) {
				throw new Error('No script options passed')
			}

			const lang = this.scriptEditorOptions?.lang ?? options.lang ?? 'bun'
			const isPreprocessor =
				this.scriptEditorOptions?.path === 'preprocessor' || options.isPreprocessor

			const userMessage =
				this.mode === 'flow'
					? prepareFlowUserMessage(oldInstructions, this.flowAiChatHelpers!.getFlowAndSelectedId())
					: this.mode === 'navigator'
						? prepareNavigatorUserMessage(oldInstructions)
						: await prepareScriptUserMessage(oldInstructions, lang, oldSelectedContext, {
								isPreprocessor
							})

			this.messages.push(userMessage)
			await this.historyManager.saveChat(this.displayMessages, this.messages)

			this.currentReply = ''

			const params: {
				messages: ChatCompletionMessageParam[]
				abortController: AbortController
				callbacks: ToolCallbacks & {
					onNewToken: (token: string) => void
					onMessageEnd: () => void
				}
			} = {
				messages: this.messages,
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
										this.mode === 'script'
											? oldSelectedContext.filter((c) => c.type === 'code')
											: undefined
								}
							]
						}
						this.currentReply = ''
					},
					onToolCall: (id, content) => {
						this.displayMessages = [
							...this.displayMessages,
							{ role: 'tool', tool_call_id: id, content }
						]
					},
					onFinishToolCall: (id, content) => {
						console.log('onFinishToolCall', id, content)
						const existingIdx = this.displayMessages.findIndex(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingIdx !== -1) {
							this.displayMessages[existingIdx].content = content
						} else {
							this.displayMessages.push({ role: 'tool', tool_call_id: id, content })
						}
					}
				}
			}

			if (this.mode === 'flow' && !this.flowAiChatHelpers) {
				throw new Error('No flow helpers found')
			}
			await chatRequest({
				...params
			})
			if (this.currentReply) {
				// just in case the onMessageEnd is not called (due to an error for instance)
				this.displayMessages = [
					...this.displayMessages,
					{
						role: 'assistant',
						content: this.currentReply,
						contextElements:
							this.mode === 'script'
								? oldSelectedContext.filter((c) => c.type === 'code')
								: undefined
					}
				]
				this.currentReply = ''
			}

			await this.historyManager.saveChat(this.displayMessages, this.messages)
		} catch (err) {
			console.error(err)
			if (err instanceof Error) {
				sendUserToast('Failed to send request: ' + err.message, true)
			} else {
				sendUserToast('Failed to send request', true)
			}
		} finally {
			this.loading = false
		}
	}

	cancel = () => {
		this.currentReply = ''
		this.abortController?.abort()
	}

	fix = () => {
		this.instructions = 'Fix the error'
		this.contextManager?.setFixContext()
		this.sendRequest()
	}

	addSelectedLinesToContext = (lines: string, startLine: number, endLine: number) => {
		this.contextManager?.addSelectedLinesToContext(lines, startLine, endLine)
	}

	saveAndClear = async () => {
		await this.historyManager.save(this.displayMessages, this.messages)
		this.displayMessages = []
		this.messages = []
	}

	loadPastChat = async (id: string) => {
		const chat = this.historyManager.loadPastChat(id)
		if (chat) {
			this.displayMessages = chat.displayMessages
			this.messages = chat.actualMessages
			this.automaticScroll = true
		}
	}

	generateStep = async (moduleId: string, lang: ScriptLang, instructions: string) => {
		if (!this.flowAiChatHelpers) {
			throw new Error('No flow helpers found')
		}
		this.flowAiChatHelpers.selectStep(moduleId)
		await this.sendRequest({
			instructions: instructions,
			mode: 'script',
			lang: lang,
			isPreprocessor: moduleId === 'preprocessor'
		})
	}
}

export const AIChatService = new AIChat()
