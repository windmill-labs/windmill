import type { AIProviderModel, ScriptLang } from '$lib/gen/types.gen'
import type { ScriptOptions } from './ContextManager.svelte'
import {
	flowTools,
	prepareFlowSystemMessage,
	prepareFlowUserMessage,
	type FlowAIChatHelpers
} from './flow/core'
import ContextManager from './ContextManager.svelte'
import HistoryManager from './HistoryManager.svelte'
import { processToolCall, type DisplayMessage, type Tool, type ToolCallbacks } from './shared'
import type {
	ChatCompletionChunk,
	ChatCompletionMessageParam,
	ChatCompletionMessageToolCall,
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { prepareScriptSystemMessage, prepareScriptTools } from './script/core'
import { navigatorTools, prepareNavigatorSystemMessage } from './navigator/core'
import { prepareScriptUserMessage } from './script/core'
import { prepareNavigatorUserMessage } from './navigator/core'
import { sendUserToast } from '$lib/toast'
import { getCompletion } from '../lib'
import { dfs } from '$lib/components/flows/previousResults'
import { getStringError } from './utils'
import type { FlowModuleState, FlowState } from '$lib/components/flows/flowState'
import type { CurrentEditor, ExtendedOpenFlow } from '$lib/components/flows/types'
import { untrack } from 'svelte'
import type { DBSchemas } from '$lib/stores'

type TriggerablesMap = Record<
	string,
	{ description: string; onTrigger: ((id: string) => void) | undefined }
>

class AIChatManager {
	DEFAULT_SIZE = 22
	NAVIGATION_SYSTEM_PROMPT = `
	CONSIDERATIONS:
	 - You are provided with a tool to switch to navigation mode, use it when the user asks you to navigate the application or help them find something.
	`

	contextManager = new ContextManager()
	historyManager = new HistoryManager()
	abortController: AbortController | undefined = undefined

	size = $state<number>(localStorage.getItem('ai-chat-open') === 'true' ? this.DEFAULT_SIZE : 0)
	instructions = $state<string>('')
	pendingPrompt = $state<string>('')
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

	open = $derived(this.size > 0)

	updateMode(currentMode: 'script' | 'flow' | 'navigator') {
		if (
			!this.allowedModes[currentMode] &&
			Object.keys(this.allowedModes).filter((k) => this.allowedModes[k]).length === 1
		) {
			const firstKey = Object.keys(this.allowedModes).filter((k) => this.allowedModes[k])[0]
			this.changeMode(firstKey as 'script' | 'flow' | 'navigator')
		}
	}

	changeMode(mode: 'script' | 'flow' | 'navigator', pendingPrompt?: string) {
		this.mode = mode
		this.pendingPrompt = pendingPrompt ?? ''
		if (mode === 'script') {
			this.systemMessage = prepareScriptSystemMessage()
			this.systemMessage.content = this.NAVIGATION_SYSTEM_PROMPT + this.systemMessage.content
			const context = this.contextManager.getSelectedContext()
			const lang = this.scriptEditorOptions?.lang ?? 'bun'
			this.tools = [this.changeModeTool, ...prepareScriptTools(lang, context)]
			this.helpers = {
				getLang: () => lang
			}
		} else if (mode === 'flow') {
			this.systemMessage = prepareFlowSystemMessage()
			this.systemMessage.content = this.NAVIGATION_SYSTEM_PROMPT + this.systemMessage.content
			this.tools = [this.changeModeTool, ...flowTools]
			this.helpers = this.flowAiChatHelpers
		} else if (mode === 'navigator') {
			this.systemMessage = prepareNavigatorSystemMessage()
			this.tools = [this.changeModeTool, ...navigatorTools]
			this.helpers = {}
		}
	}

	canApplyCode = $derived(this.allowedModes.script && this.mode === 'script')

	private changeModeTool = {
		def: {
			type: 'function' as const,
			function: {
				name: 'change_mode',
				description:
					'Change the AI mode to the one specified. Script mode is used to create scripts, and flow mode is used to create flows. Navigator mode is used to navigate the application and help the user find what they are looking for.',
				parameters: {
					type: 'object',
					properties: {
						mode: {
							type: 'string',
							description: 'The mode to change to',
							enum: ['script', 'flow', 'navigator']
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
			toolCallbacks.onToolCall(toolId, 'Switching to ' + args.mode + ' mode...')
			this.changeMode(args.mode as 'script' | 'flow' | 'navigator', args.pendingPrompt)
			toolCallbacks.onFinishToolCall(toolId, 'Switched to ' + args.mode + ' mode')
			return 'Mode changed to ' + args.mode
		}
	}

	openChat = () => {
		this.size = this.DEFAULT_SIZE
		localStorage.setItem('ai-chat-open', 'true')
	}

	closeChat = () => {
		this.size = 0
		localStorage.setItem('ai-chat-open', 'false')
	}

	toggleOpen = () => {
		this.size = this.size === 0 ? this.DEFAULT_SIZE : 0
		localStorage.setItem('ai-chat-open', this.size === 0 ? 'false' : 'true')
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

	private chatRequest = async ({
		messages,
		abortController,
		callbacks
	}: {
		messages: ChatCompletionMessageParam[]
		abortController: AbortController
		callbacks: ToolCallbacks & {
			onNewToken: (token: string) => void
			onMessageEnd: () => void
		}
	}) => {
		try {
			let completion: any = null

			while (true) {
				const systemMessage = this.systemMessage
				const tools = this.tools
				const helpers = this.helpers

				let pendingPrompt = this.pendingPrompt
				let pendingUserMessage: ChatCompletionUserMessageParam | undefined = undefined
				if (pendingPrompt) {
					if (this.mode === 'script') {
						pendingUserMessage = await prepareScriptUserMessage(
							pendingPrompt,
							this.scriptEditorOptions?.lang as ScriptLang | 'bunnative',
							this.contextManager.getSelectedContext()
						)
					} else if (this.mode === 'flow') {
						pendingUserMessage = prepareFlowUserMessage(
							pendingPrompt,
							this.flowAiChatHelpers!.getFlowAndSelectedId()
						)
					} else if (this.mode === 'navigator') {
						pendingUserMessage = prepareNavigatorUserMessage(pendingPrompt)
					}
					this.pendingPrompt = ''
				}
				completion = await getCompletion(
					[systemMessage, ...messages, ...(pendingUserMessage ? [pendingUserMessage] : [])],
					abortController,
					tools.map((t) => t.def)
				)

				if (completion) {
					const finalToolCalls: Record<number, ChatCompletionChunk.Choice.Delta.ToolCall> = {}

					let answer = ''
					for await (const chunk of completion) {
						if (!('choices' in chunk && chunk.choices.length > 0 && 'delta' in chunk.choices[0])) {
							continue
						}
						const c = chunk as ChatCompletionChunk
						const delta = c.choices[0].delta.content
						if (delta) {
							answer += delta
							callbacks.onNewToken(delta)
						}
						const toolCalls = c.choices[0].delta.tool_calls || []
						for (const toolCall of toolCalls) {
							const { index } = toolCall
							const finalToolCall = finalToolCalls[index]
							if (!finalToolCall) {
								finalToolCalls[index] = toolCall
							} else {
								if (toolCall.function?.arguments) {
									if (!finalToolCall.function) {
										finalToolCall.function = toolCall.function
									} else {
										finalToolCall.function.arguments =
											(finalToolCall.function.arguments ?? '') + toolCall.function.arguments
									}
								}
							}
						}
					}

					if (answer) {
						messages.push({ role: 'assistant', content: answer })
					}

					callbacks.onMessageEnd()

					const toolCalls = Object.values(finalToolCalls).filter(
						(toolCall) => toolCall.id !== undefined && toolCall.function?.arguments !== undefined
					) as ChatCompletionMessageToolCall[]

					if (toolCalls.length > 0) {
						messages.push({
							role: 'assistant',
							tool_calls: toolCalls.map((t) => ({
								...t,
								function: {
									...t.function,
									arguments: t.function.arguments || '{}'
								}
							}))
						})
						for (const toolCall of toolCalls) {
							await processToolCall({
								tools,
								toolCall,
								messages,
								helpers,
								toolCallbacks: callbacks
							})
						}
					} else {
						break
					}
				}
			}
			return messages
		} catch (err) {
			if (!abortController.signal.aborted) {
				throw err
			} else {
				return messages
			}
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
			this.changeMode(options.mode, '')
		} else {
			this.changeMode(this.mode, '')
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
			await this.chatRequest({
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

	listenForScriptEditorContextChange = (
		dbSchemas: DBSchemas,
		workspaceStore: string | undefined,
		copilotSessionModel: AIProviderModel | undefined
	) => {
		if (this.scriptEditorOptions) {
			this.contextManager.updateAvailableContext(
				this.scriptEditorOptions,
				dbSchemas,
				workspaceStore ?? '',
				!copilotSessionModel?.model.endsWith('/thinking'),
				untrack(() => this.contextManager.getSelectedContext())
			)
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
		} else {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
		}

		return () => {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
		}
	}

	listenForSelectedIdChanges = (
		selectedId: string,
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
}

export const aiChatManager = new AIChatManager()
