import OpenAI from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { getCompletion, parseOpenAICompletion, providerSupportsWebSearch } from '../lib'
import {
	resolveEffectiveReasoning,
	resolveRequestReasoning,
	type ReasoningProviderModel
} from '../reasoningRegistry'
import { getAnthropicCompletion, parseAnthropicCompletion } from './anthropic'
import { modelSupportsVision, usesAnthropicMessagesApi } from '../modelConfig'
import { boundImagePartBytes, stripImagePartsFromMessages } from './imageUtils'
import {
	buildPromptCacheKey,
	getOpenAIResponsesCompletion,
	parseOpenAIResponsesCompletion
} from './openai-responses'
import type { Tool, ToolCallbacks } from './shared'
import { sanitizeToolCallArguments } from './toolCallArguments'
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
	onWebSearchUnavailable?: () => void
	/**
	 * Called when the provider refuses to generate reasoning summaries (OpenAI
	 * gates them behind organization verification). The request is retried
	 * without summaries, so reasoning happens but stays hidden.
	 */
	onReasoningSummaryUnavailable?: () => void
	/** Return a pending user message to inject between iterations, or undefined. */
	getPendingUserMessage?: () => ChatCompletionUserMessageParam | undefined
	/**
	 * Optional caller-owned accumulator for the messages produced this run —
	 * lets the caller recover partial output if the loop throws or is aborted.
	 */
	addedMessages?: ChatCompletionMessageParam[]
	/** Called before each iteration (e.g. to refresh tool schemas, or to record
	 * which model the iteration is about to use). */
	onBeforeIteration?: (
		tools: Tool<any>[],
		helpers: any,
		modelProvider: ReasoningProviderModel
	) => Promise<void>
	/** Fired for each completed provider response, before the loop continues. The
	 * loop can fail or be aborted at any iteration, so spend has to be handed over
	 * as it happens — a callback only at the end would discard everything the
	 * earlier iterations were already billed for. */
	onUsage?: (usage: ChatTokenUsage, modelProvider: ReasoningProviderModel) => void
}

export interface ChatLoopResult {
	addedMessages: ChatCompletionMessageParam[]
	/** Sum of usage across all loop iterations. */
	tokenUsage: ChatTokenUsage
	lastIterationUsage: ChatTokenUsage | null
	hitMaxIterations: boolean
}

/**
 * Returns the longest prefix of `messages` that forms a valid request sequence:
 * every assistant `tool_calls` batch must be fully answered by following tool
 * messages before the next assistant turn. Used to commit the partial output of
 * an aborted or failed turn as context for a follow-up, without leaving a
 * dangling tool_call (which the provider APIs reject on the next request).
 */
export function truncateToToolPairedPrefix(
	messages: ChatCompletionMessageParam[]
): ChatCompletionMessageParam[] {
	let lastValidLen = 0
	let pending = new Set<string>()
	for (let i = 0; i < messages.length; i++) {
		const m = messages[i]
		if (m.role === 'assistant') {
			// A new assistant turn while the previous tool batch is unanswered would
			// be invalid — stop at the last known-good boundary.
			if (pending.size > 0) break
			const toolCalls = m.tool_calls ?? []
			if (toolCalls.length === 0) {
				lastValidLen = i + 1
			} else {
				pending = new Set(toolCalls.map((c) => c.id))
			}
		} else if (m.role === 'tool') {
			pending.delete(m.tool_call_id)
			// Boundary is valid only once every tool_call in the batch is answered.
			if (pending.size === 0) lastValidLen = i + 1
		} else {
			// user/system message: a valid boundary only if no tool calls are pending.
			if (pending.size > 0) break
			lastValidLen = i + 1
		}
	}
	return messages.slice(0, lastValidLen)
}

/**
 * Like `truncateToToolPairedPrefix`, but closes the batch it would have dropped:
 * every call in the trailing `tool_calls` message still missing a result gets one
 * saying it was interrupted. A batch's calls execute one at a time, so truncating
 * it discards steps that already finished — and whose side effects are already
 * real. For a snapshot that must outlive its turn, keeping them beats a clean cut.
 */
export function closeInterruptedToolBatch(
	messages: ChatCompletionMessageParam[],
	interruptedContent: string
): ChatCompletionMessageParam[] {
	const paired = truncateToToolPairedPrefix(messages)
	const rest = messages.slice(paired.length)
	const batch = rest[0]
	if (batch?.role !== 'assistant' || !batch.tool_calls?.length) return paired
	// Results for THIS batch only, stopping at the first non-tool message:
	// anything past a still-pending batch was never valid context.
	const batchIds = new Set(batch.tool_calls.map((c) => c.id))
	const answers: ChatCompletionMessageParam[] = []
	const answered = new Set<string>()
	for (const m of rest.slice(1)) {
		if (m.role !== 'tool') break
		if (!batchIds.has(m.tool_call_id)) continue
		answers.push(m)
		answered.add(m.tool_call_id)
	}
	const interrupted = [...batchIds]
		.filter((id) => !answered.has(id))
		.map((id) => ({ role: 'tool' as const, tool_call_id: id, content: interruptedContent }))
	return [...paired, batch, ...answers, ...interrupted]
}

const unsupportedWebSearchCache = new Set<string>()
const WEB_SEARCH_UNAVAILABLE_STATUS_CODES = new Set([400, 403, 404])

// Reasoning-summary availability is an org-level property of the provider
// credentials (OpenAI organization verification), not of the model — key by
// workspace + provider. In-memory on purpose: a page reload re-probes, so a
// freshly verified organization starts getting summaries again.
const unsupportedReasoningSummaryCache = new Set<string>()
const REASONING_SUMMARY_UNAVAILABLE_STATUS_CODES = new Set([400, 403])

// A gateway that validates the request body strictly rejects `prompt_cache_key`
// outright, so it is a property of the endpoint the credentials point at, not of the
// model. Same in-memory reasoning as above: a reload re-probes.
const unsupportedPromptCacheKeyCache = new Set<string>()

function getWebSearchCacheKey(workspace: string, modelProvider: ReasoningProviderModel): string {
	return [workspace, modelProvider.provider, modelProvider.model].join(':')
}

function getReasoningSummaryCacheKey(
	workspace: string,
	modelProvider: ReasoningProviderModel
): string {
	return [workspace, modelProvider.provider].join(':')
}

// Keyed without the model, like the reasoning-summary probe: a body-validation refusal
// belongs to the endpoint, so switching models must not re-pay the failed round trip.
function getPromptCacheKeySupportKey(
	workspace: string,
	modelProvider: ReasoningProviderModel
): string {
	return [workspace, modelProvider.provider].join(':')
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null
}

function appendTextPart(parts: string[], value: unknown) {
	if (typeof value === 'string' && value.trim()) {
		parts.push(value)
	}
}

function getErrorText(err: unknown): string {
	const parts: string[] = []
	if (err instanceof Error) {
		appendTextPart(parts, err.message)
	}
	if (typeof err === 'string') {
		appendTextPart(parts, err)
	}
	if (isRecord(err)) {
		appendTextPart(parts, err.message)
		appendTextPart(parts, err.type)
		appendTextPart(parts, err.code)
		appendTextPart(parts, err.param)

		const nested = err.error
		if (isRecord(nested)) {
			appendTextPart(parts, nested.message)
			appendTextPart(parts, nested.type)
			appendTextPart(parts, nested.code)
			appendTextPart(parts, nested.param)
		} else {
			appendTextPart(parts, nested)
		}
	}
	if (parts.length > 0) {
		return parts.join(' ')
	}
	try {
		return JSON.stringify(err)
	} catch {
		return String(err)
	}
}

function getErrorStatus(err: unknown): number | undefined {
	if (!isRecord(err)) {
		return undefined
	}
	const candidates = [err.status]
	if (isRecord(err.response)) {
		candidates.push(err.response.status)
	}
	if (isRecord(err.error)) {
		candidates.push(err.error.status)
	}
	return candidates.find((status): status is number => typeof status === 'number')
}

function hasWebSearchUnavailableSignal(err: unknown): boolean {
	const message = getErrorText(err).toLowerCase()
	const webSearchTerm = '(?:web[_ -]?search|web search|web-search)'
	const unavailableTerm =
		'(?:not supported|unsupported|not available|unavailable|disabled|not enabled|enable web search|forbidden|not permitted|permission|policy|blocked|access)'
	const patterns = [
		new RegExp(`${webSearchTerm}.*${unavailableTerm}`),
		new RegExp(`${unavailableTerm}.*${webSearchTerm}`),
		/\bmust\s+enable\s+web[_ -]?search\b/,
		/\bweb search options\b.*\bnot supported\b/,
		/\bhosted tools?\b.*\b(?:not supported|unsupported)\b/,
		/\bhosted tool ['"]web_search(?:_preview)?['"].*\b(?:not supported|unsupported)\b/
	]
	return patterns.some((pattern) => pattern.test(message))
}

function shouldRetryWithoutWebSearch(err: unknown): boolean {
	if (!hasWebSearchUnavailableSignal(err)) {
		return false
	}
	const status = getErrorStatus(err)
	return status === undefined || WEB_SEARCH_UNAVAILABLE_STATUS_CODES.has(status)
}

function getErrorParam(err: unknown): string | undefined {
	if (!isRecord(err)) {
		return undefined
	}
	const candidates = [err.param]
	if (isRecord(err.error)) {
		candidates.push(err.error.param)
	}
	return candidates.find((param): param is string => typeof param === 'string')
}

// Unverified OpenAI organizations get a 400 on the reasoning.summary param
// ("Your organization must be verified to generate reasoning summaries").
function shouldRetryWithoutReasoningSummary(err: unknown): boolean {
	const status = getErrorStatus(err)
	if (status !== undefined && !REASONING_SUMMARY_UNAVAILABLE_STATUS_CODES.has(status)) {
		return false
	}
	if (getErrorParam(err) === 'reasoning.summary') {
		return true
	}
	const message = getErrorText(err).toLowerCase()
	return (
		message.includes('reasoning.summary') ||
		/verified to (?:generate|stream) reasoning summar/.test(message)
	)
}

// An OpenAI-compatible gateway that validates the body strictly names the offending
// field, whether it calls it an unrecognized argument or an unexpected additional
// property.
function shouldRetryWithoutPromptCacheKey(err: unknown): boolean {
	const status = getErrorStatus(err)
	if (status !== undefined && status !== 400) {
		return false
	}
	if (getErrorParam(err) === 'prompt_cache_key') {
		return true
	}
	return getErrorText(err).includes('prompt_cache_key')
}

function markPromptCacheKeyUnsupported(cacheKey: string, err: unknown) {
	unsupportedPromptCacheKeyCache.add(cacheKey)
	console.warn('prompt_cache_key rejected; retrying without prompt caching hints:', err)
}

function markReasoningSummaryUnsupported(
	cacheKey: string,
	err: unknown,
	onReasoningSummaryUnavailable?: () => void
) {
	unsupportedReasoningSummaryCache.add(cacheKey)
	console.warn('Reasoning summaries unavailable; retrying without them:', err)
	onReasoningSummaryUnavailable?.()
}

function markWebSearchUnsupported(
	cacheKey: string,
	err: unknown,
	onWebSearchUnavailable?: () => void
) {
	unsupportedWebSearchCache.add(cacheKey)
	console.warn('Native web search unavailable; retrying without web search:', err)
	onWebSearchUnavailable?.()
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
		onReasoningSummaryUnavailable,
		getPendingUserMessage,
		onBeforeIteration
	} = config
	let skipResponsesApi = config.skipResponsesApi ?? false

	const addedMessages: ChatCompletionMessageParam[] = config.addedMessages ?? []
	let tokenUsage = emptyChatTokenUsage()
	let lastIterationUsage: ChatTokenUsage | null = null
	let iterations = 0
	let hitMaxIterations = false
	// The model of the iteration currently in flight; re-read per iteration like
	// `config.modelProvider` itself, so usage is attributed to the model that
	// actually served it rather than to whatever is selected when the loop ends.
	let iterationModel: ReasoningProviderModel | undefined

	// Reported as the provider's usage arrives, not when the parser returns: a parser
	// waits on tool execution, which can wait on a person, and a tab closed in that
	// gap would drop a response that was already billed. Accounting for the turn's
	// own totals stays on the return path, where every parser reports uniformly.
	const reportUsage = (usage: ChatTokenUsage | null | undefined) => {
		if (usage && iterationModel) {
			config.onUsage?.(usage, iterationModel)
		}
	}

	const trackUsage = (usage: ChatTokenUsage | null | undefined) => {
		tokenUsage = addChatTokenUsage(tokenUsage, usage)
		// Some providers/paths report no usage (prompt 0); keep the last real one.
		if (usage && usage.prompt > 0) {
			lastIterationUsage = usage
		}
	}

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
		iterationModel = modelProvider
		const webSearchCacheKey = getWebSearchCacheKey(workspace, modelProvider)
		const webSearch =
			(config.webSearch ?? true) &&
			providerSupportsWebSearch(modelProvider.provider) &&
			!unsupportedWebSearchCache.has(webSearchCacheKey)

		if (onBeforeIteration) {
			await onBeforeIteration(tools, helpers, modelProvider)
		}

		const pendingUserMessage = getPendingUserMessage?.()

		const isOpenAI =
			modelProvider.provider === 'openai' || modelProvider.provider === 'azure_openai'
		const isAnthropic = usesAnthropicMessagesApi(modelProvider.provider, modelProvider.model)
		// Resolve effort once in chat context (applies the default-on level for
		// capable models, and the provider-native disable token for an explicit
		// off on reasoning-by-default providers); passed explicitly to each seam
		// so background paths (metadata/autocomplete) never inherit it.
		const reasoningEffort = resolveRequestReasoning(modelProvider)

		// Checked per iteration, like the model itself: the selector stays enabled
		// while the loop runs, and a switch to a known text-only model mid-turn
		// would otherwise send it the history's image parts and fail the turn.
		// The byte bound is also per iteration because screenshots taken by tools
		// grow the history mid-loop (see MAX_TOTAL_IMAGE_BYTES).
		const visibleMessages = modelSupportsVision(modelProvider.provider, modelProvider.model)
			? boundImagePartBytes(messages)
			: stripImagePartsFromMessages(messages)
		const messageParams = [
			systemMessage,
			...sanitizeToolCallArguments(visibleMessages),
			...(pendingUserMessage ? [pendingUserMessage] : [])
		]
		const toolDefs = tools.map((t) => t.def)
		const parseOptions = {
			workspace,
			provider: modelProvider.provider,
			onTokenUsage: reportUsage
		}

		if (isOpenAI) {
			const reasoningSummaryCacheKey = getReasoningSummaryCacheKey(workspace, modelProvider)
			// Gate on the effective (not request) reasoning: an explicit off resolves
			// to a truthy disable token like 'none' on the request side, and asking
			// for a summary on a non-reasoning request would 400 on unverified orgs.
			let reasoningSummary =
				resolveEffectiveReasoning(modelProvider) !== undefined &&
				!unsupportedReasoningSummaryCache.has(reasoningSummaryCacheKey)

			// One key for the whole chat surface: every iteration opens with the same
			// system prompt and tool definitions, and each one extends the previous
			// iteration's prefix, so they all belong on the same cache.
			const promptCacheKey = buildPromptCacheKey('chat', modelProvider, workspace)
			const promptCacheSupportKey = getPromptCacheKeySupportKey(workspace, modelProvider)
			let usePromptCacheKey = !unsupportedPromptCacheKeyCache.has(promptCacheSupportKey)

			const runOpenAIResponses = async (useWebSearch: boolean): Promise<boolean> => {
				const completion = await getOpenAIResponsesCompletion(
					messageParams,
					abortController,
					toolDefs,
					{
						forceModelProvider: modelProvider,
						openaiClient: clients.openai,
						webSearch: useWebSearch,
						reasoningEffort,
						reasoningSummary,
						promptCacheKey: usePromptCacheKey ? promptCacheKey : undefined
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
				trackUsage(continueCompletion.tokenUsage)
				return continueCompletion.shouldContinue
			}

			let useCompletionsApi = skipResponsesApi
			if (!skipResponsesApi) {
				// Retry the Responses call disabling whichever optional feature the
				// provider rejected (reasoning summary, web search, prompt cache key) in
				// the order the errors arrive — a turn can hit several, any one first.
				// Each retry permanently disables one feature, so this loops at most
				// once per feature.
				let useWebSearch = webSearch
				let outcome: 'break' | 'continue' | undefined
				let fallbackError: unknown
				while (outcome === undefined) {
					try {
						outcome = (await runOpenAIResponses(useWebSearch)) ? 'continue' : 'break'
					} catch (err) {
						if (reasoningSummary && shouldRetryWithoutReasoningSummary(err)) {
							markReasoningSummaryUnsupported(
								reasoningSummaryCacheKey,
								err,
								onReasoningSummaryUnavailable
							)
							reasoningSummary = false
						} else if (useWebSearch && shouldRetryWithoutWebSearch(err)) {
							markWebSearchUnsupported(webSearchCacheKey, err, config.onWebSearchUnavailable)
							useWebSearch = false
						} else if (usePromptCacheKey && shouldRetryWithoutPromptCacheKey(err)) {
							markPromptCacheKeyUnsupported(promptCacheSupportKey, err)
							usePromptCacheKey = false
						} else {
							fallbackError = err
							break
						}
					}
				}
				if (outcome === 'break') {
					break
				}
				if (outcome === 'continue') {
					continue
				}

				console.warn('OpenAI Responses API failed, falling back to Completions API:', fallbackError)
				const errorMessage = getErrorText(fallbackError)
				if (errorMessage.includes('Responses API is not enabled')) {
					skipResponsesApi = true
					onSkipResponsesApi?.()
				}
				useCompletionsApi = true
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
				trackUsage(continueCompletion.tokenUsage)
				if (!continueCompletion.shouldContinue) {
					break
				}
			}
		} else if (isAnthropic) {
			const runAnthropic = async (useWebSearch: boolean): Promise<boolean> => {
				const completion = await getAnthropicCompletion(messageParams, abortController, toolDefs, {
					forceModelProvider: modelProvider,
					anthropicClient: clients.anthropic,
					webSearch: useWebSearch,
					reasoningEffort
				})
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
				trackUsage(continueCompletion.tokenUsage)
				return continueCompletion.shouldContinue
			}

			try {
				if (!(await runAnthropic(webSearch))) {
					break
				}
			} catch (err) {
				if (webSearch && shouldRetryWithoutWebSearch(err)) {
					markWebSearchUnsupported(webSearchCacheKey, err, config.onWebSearchUnavailable)
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
				reasoningEffort,
				promptCaching: true
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
				trackUsage(continueCompletion.tokenUsage)
				if (!continueCompletion.shouldContinue) {
					break
				}
			}
		}
	}

	return { addedMessages, tokenUsage, lastIterationUsage, hitMaxIterations }
}
