import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import type { AIProviderModel } from '$lib/gen'
import { getCompletion, parseOpenAICompletion } from '../lib'
import { getAnthropicCompletion, parseAnthropicCompletion } from './anthropic'
import {
	getOpenAIResponsesCompletion,
	parseOpenAIResponsesCompletion
} from './openai-responses'
import type { Tool, ToolCallbacks } from './shared'

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
	modelProvider: AIProviderModel
	clients: ChatClients
	workspace: string
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
	let iterations = 0

	while (true) {
		if (maxIterations !== undefined && iterations >= maxIterations) {
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

		if (onBeforeIteration) {
			await onBeforeIteration(tools, helpers)
		}

		const pendingUserMessage = getPendingUserMessage?.()

		const isOpenAI =
			modelProvider.provider === 'openai' || modelProvider.provider === 'azure_openai'
		const isAnthropic = modelProvider.provider === 'anthropic'

		const messageParams = [
			systemMessage,
			...messages,
			...(pendingUserMessage ? [pendingUserMessage] : [])
		]
		const toolDefs = tools.map((t) => t.def)
		const parseOptions = { workspace }

		if (isOpenAI) {
			let useCompletionsApi = skipResponsesApi
			if (!skipResponsesApi) {
				try {
					const completion = await getOpenAIResponsesCompletion(
						messageParams,
						abortController,
						toolDefs,
						{
							forceModelProvider: modelProvider,
							openaiClient: clients.openai
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
					if (!continueCompletion) {
						break
					}
				} catch (err) {
					console.warn(
						'OpenAI Responses API failed, falling back to Completions API:',
						err
					)
					const errorMessage = err instanceof Error ? err.message : String(err)
					if (errorMessage.includes('Responses API is not enabled')) {
						skipResponsesApi = true
						onSkipResponsesApi?.()
					}
					useCompletionsApi = true
				}
			}

			if (useCompletionsApi) {
				const completion = await getCompletion(messageParams, abortController, toolDefs, {
					forceCompletions: true,
					forceModelProvider: modelProvider,
					openaiClient: clients.openai
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
				if (!continueCompletion) {
					break
				}
			}
		} else if (isAnthropic) {
			const completion = await getAnthropicCompletion(
				messageParams,
				abortController,
				toolDefs,
				{
					forceModelProvider: modelProvider,
					anthropicClient: clients.anthropic
				}
			)
			if (completion) {
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
				if (!continueCompletion) {
					break
				}
			}
		} else {
			const completion = await getCompletion(messageParams, abortController, toolDefs, {
				forceModelProvider: modelProvider,
				openaiClient: clients.openai
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
				if (!continueCompletion) {
					break
				}
			}
		}
	}

	return { addedMessages }
}
