import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { getCompletion, parseOpenAICompletion, providerSupportsWebSearch } from '../lib'
import { resolveEffectiveReasoning, type ReasoningProviderModel } from '../reasoningRegistry'
import { getAnthropicCompletion, parseAnthropicCompletion } from './anthropic'
import { getOpenAIResponsesCompletion, parseOpenAIResponsesCompletion } from './openai-responses'
import type { Tool, ToolCallbacks } from './shared'
import { addChatTokenUsage, emptyChatTokenUsage, type ChatTokenUsage } from './tokenUsage'

export interface ChatClients {
	openai: OpenAI
	anthropic: Anthropic
}

export interface ChatLoopConfig {
	messages: ChatCompletionMessageParam[]
	/**
	 * System message, tools, helpers, and modelProvider are re-read from this config
	 * on every iteration. Callers can use JS getters to provide dynamic values
	 * (e.g. AIChatManager uses getters so mode changes mid-loop take effect).
	 */
	systemMessage: ChatCompletionSystemMessageParam
	tools: Tool<any>[]
	helpers: any
	abortController: AbortController
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	}
	modelProvider: ReasoningProviderModel
	clients: ChatClients
	workspace: string
	/**
	 * Enable provider-native web search. Defaults to true for compatible providers
	 * and is re-read each iteration so model/provider changes take effect. Explicit
	 * true is still ignored for providers without native web search support.
	 */
	webSearch?: boolean
	/** Maximum iterations for the loop. undefined = unlimited (production). */
	maxIterations?: number
	skipResponsesApi?: boolean
	onSkipResponsesApi?: () => void
	/** Return a pending user message to inject between iterations, or undefined. */
	getPendingUserMessage?: () => ChatCompletionUserMessageParam | undefined
	/** Called before each iteration (e.g. to refresh tool schemas). */
	onBeforeIteration?: (tools: Tool<any>[], helpers: any) => Promise<void>
}

export interface ChatLoopResult {
	addedMessages: ChatCompletionMessageParam[]
	tokenUsage: ChatTokenUsage
	hitMaxIterations: boolean
}

const WEB_SEARCH_UNAVAILABLE_MESSAGE =
	'You can disable websearch in your workspace settings.'
const unsupportedWebSearchCache = new Set<string>()

function getWebSearchCacheKey(workspace: string, modelProvider: ReasoningProviderModel): string {
	return [workspace, modelProvider.provider, modelProvider.model].join(':')
}

function getErrorText(err: unknown): string {
	if (err instanceof Error) {
		return err.message
	}
	if (typeof err === 'string') {
		return err
	}
	try {
		return JSON.stringify(err)
	} catch {
		return String(err)
	}
}

function shouldRetryWithoutWebSearch(err: unknown): boolean {
	const message = getErrorText(err).toLowerCase()
	return (
		message.includes('web_search') ||
		message.includes('web search') ||
		(message.includes('tool') &&
			(message.includes('unsupported') ||
				message.includes('not supported') ||
				message.includes('not enabled') ||
				message.includes('disabled') ||
				message.includes('unknown')))
	)
}

function markWebSearchUnsupported(
	callbacks: ToolCallbacks & { onMessageEnd: () => void },
	cacheKey: string,
	err: unknown
) {
	unsupportedWebSearchCache.add(cacheKey)
	console.warn('Native web search unavailable; retrying without web search:', err)
	callbacks.onMessageEnd()
	callbacks.setToolStatus(`web_search_unavailable:${cacheKey}`, {
		content: WEB_SEARCH_UNAVAILABLE_MESSAGE,
		error: WEB_SEARCH_UNAVAILABLE_MESSAGE,
		isLoading: false,
		isStreamingArguments: false,
		needsConfirmation: false,
		toolName: 'web_search',
		showDetails: false,
		autoCollapseDetails: true
	})
}

export async function runChatLoop(config: ChatLoopConfig): Promise<ChatLoopResult> {
	const {
		messages,
		abortController,
		callbacks,
		clients,
		workspace,
		maxIterations,
		onSkipResponsesApi,
		getPendingUserMessage,
		onBeforeIteration
	} = config
	let skipResponsesApi = config.skipResponsesApi ?? false

	const addedMessages: ChatCompletionMessageParam[] = []
	let tokenUsage = emptyChatTokenUsage()
	let iterations = 0
	let hitMaxIterations = false

	while (true) {
		if (maxIterations !== undefined && iterations >= maxIterations) {
			hitMaxIterations = true
			break
		}
		iterations++

		// Re-read these from config each iteration so that mode changes
		// (e.g. changeModeTool in Navigator) take effect immediately.
		// Callers can use JS getter properties to provide dynamic values.
		const tools = config.tools
		const helpers = config.helpers
		const systemMessage = config.systemMessage
		const modelProvider = config.modelProvider
		const webSearchCacheKey = getWebSearchCacheKey(workspace, modelProvider)
		const webSearch =
			(config.webSearch ?? true) &&
			providerSupportsWebSearch(modelProvider.provider) &&
			!unsupportedWebSearchCache.has(webSearchCacheKey)

		if (onBeforeIteration) {
			await onBeforeIteration(tools, helpers)
		}

		const pendingUserMessage = getPendingUserMessage?.()

		const isOpenAI =
			modelProvider.provider === 'openai' || modelProvider.provider === 'azure_openai'
		const isAnthropic = modelProvider.provider === 'anthropic'
		// Resolve effort once in chat context (applies the default-on level for
		// capable models); passed explicitly to each seam so background paths
		// (metadata/autocomplete) never inherit it.
		const reasoningEffort = resolveEffectiveReasoning(modelProvider)

		const messageParams = [
			systemMessage,
			...messages,
			...(pendingUserMessage ? [pendingUserMessage] : [])
		]
		const toolDefs = tools.map((t) => t.def)
		const parseOptions = { workspace }

		if (isOpenAI) {
			const runOpenAIResponses = async (useWebSearch: boolean): Promise<boolean> => {
				const completion = await getOpenAIResponsesCompletion(
					messageParams,
					abortController,
					toolDefs,
					{
						forceModelProvider: modelProvider,
						openaiClient: clients.openai,
						webSearch: useWebSearch,
						reasoningEffort
					}
				)
				const continueCompletion = await parseOpenAIResponsesCompletion(
					completion,
					callbacks,
					messages,
					addedMessages,
					tools,
					helpers,
					parseOptions
				)
				tokenUsage = addChatTokenUsage(tokenUsage, continueCompletion.tokenUsage)
				return continueCompletion.shouldContinue
			}

			let useCompletionsApi = skipResponsesApi
			if (!skipResponsesApi) {
				try {
					if (!(await runOpenAIResponses(webSearch))) {
						break
					}
				} catch (err) {
					let fallbackError = err
					if (webSearch && shouldRetryWithoutWebSearch(err)) {
						markWebSearchUnsupported(callbacks, webSearchCacheKey, err)
						try {
							if (!(await runOpenAIResponses(false))) {
								break
							}
							continue
						} catch (retryErr) {
							fallbackError = retryErr
						}
					}

					console.warn(
						'OpenAI Responses API failed, falling back to Completions API:',
						fallbackError
					)
					const errorMessage = getErrorText(fallbackError)
					if (errorMessage.includes('Responses API is not enabled')) {
						skipResponsesApi = true
						onSkipResponsesApi?.()
					}
					useCompletionsApi = true
				}
			}

			if (useCompletionsApi) {
				if (webSearch) {
					console.warn(
						'Web search is only supported via the OpenAI Responses API; ignoring it for the Completions API fallback.'
					)
				}
				const completion = await getCompletion(messageParams, abortController, toolDefs, {
					forceCompletions: true,
					forceModelProvider: modelProvider,
					openaiClient: clients.openai,
					reasoningEffort
				})
				const continueCompletion = await parseOpenAICompletion(
					completion,
					callbacks,
					messages,
					addedMessages,
					tools,
					helpers,
					undefined,
					parseOptions
				)
				tokenUsage = addChatTokenUsage(tokenUsage, continueCompletion.tokenUsage)
				if (!continueCompletion.shouldContinue) {
					break
				}
			}
		} else if (isAnthropic) {
			const runAnthropic = async (useWebSearch: boolean): Promise<boolean> => {
				const completion = await getAnthropicCompletion(
					messageParams,
					abortController,
					toolDefs,
					{
						forceModelProvider: modelProvider,
						anthropicClient: clients.anthropic,
						webSearch: useWebSearch,
						reasoningEffort
					}
				)
				if (!completion) {
					return true
				}
				const continueCompletion = await parseAnthropicCompletion(
					completion,
					callbacks,
					messages,
					addedMessages,
					tools,
					helpers,
					abortController,
					parseOptions
				)
				tokenUsage = addChatTokenUsage(tokenUsage, continueCompletion.tokenUsage)
				return continueCompletion.shouldContinue
			}

			try {
				if (!(await runAnthropic(webSearch))) {
					break
				}
			} catch (err) {
				if (webSearch && shouldRetryWithoutWebSearch(err)) {
					markWebSearchUnsupported(callbacks, webSearchCacheKey, err)
					if (!(await runAnthropic(false))) {
						break
					}
				} else {
					throw err
				}
			}
		} else {
			const completion = await getCompletion(messageParams, abortController, toolDefs, {
				forceModelProvider: modelProvider,
				openaiClient: clients.openai,
				reasoningEffort
			})
			if (completion) {
				const continueCompletion = await parseOpenAICompletion(
					completion,
					callbacks,
					messages,
					addedMessages,
					tools,
					helpers,
					undefined,
					parseOptions
				)
				tokenUsage = addChatTokenUsage(tokenUsage, continueCompletion.tokenUsage)
				if (!continueCompletion.shouldContinue) {
					break
				}
			}
		}
	}

	return { addedMessages, tokenUsage, hitMaxIterations }
}
