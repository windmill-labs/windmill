import { OpenAI } from 'openai'
import Anthropic from '@anthropic-ai/sdk'
import type {
	ChatCompletionMessageParam,
	ChatCompletionMessageFunctionToolCall
} from 'openai/resources/index.mjs'
import type {
	MessageParam,
	TextBlockParam,
	ToolUnion,
	Tool as AnthropicTool,
	Message,
	RawMessageStreamEvent
} from '@anthropic-ai/sdk/resources'
import type { MessageStream } from '@anthropic-ai/sdk/lib/MessageStream'
import type { AIProviderModel } from '$lib/gen'
import { getProviderAndCompletionConfig, workspaceAIClients } from '../lib'
import { applyReasoningToConfig } from '../reasoningRegistry'
import {
	appendPendingToolImages,
	processToolCall,
	type Tool,
	type ToolCallbacks,
	type WebSearchSource
} from './shared'
import { anthropicUsageToChatTokenUsage, type ChatTokenUsage } from './tokenUsage'
import { parseImageDataUrl } from './imageUtils'

const ANTHROPIC_IMAGE_MEDIA_TYPES = new Set(['image/jpeg', 'image/png', 'image/gif', 'image/webp'])

/**
 * Convert an OpenAI user-message content array (text + image_url parts) to Anthropic
 * content blocks. Returns a plain string when the content is a lone text part so
 * simple messages stay unchanged. Non-image/text parts are dropped.
 */
function openAIUserContentToAnthropic(content: unknown): string | any[] {
	if (typeof content === 'string') return content
	if (!Array.isArray(content)) return JSON.stringify(content)
	const blocks: any[] = []
	for (const part of content) {
		if (part?.type === 'text' && typeof part.text === 'string') {
			blocks.push({ type: 'text', text: part.text })
		} else if (part?.type === 'image_url' && part.image_url?.url) {
			const { mediaType, base64 } = parseImageDataUrl(part.image_url.url)
			if (!base64) continue
			blocks.push({
				type: 'image',
				source: {
					type: 'base64',
					media_type: ANTHROPIC_IMAGE_MEDIA_TYPES.has(mediaType) ? mediaType : 'image/png',
					data: base64
				}
			})
		}
	}
	if (blocks.length === 1 && blocks[0].type === 'text') return blocks[0].text
	return blocks
}

interface ParsedCompletionResult {
	shouldContinue: boolean
	tokenUsage: ChatTokenUsage
}

type WebSearchStatus = 'searching' | 'completed' | 'failed'

function setAnthropicWebSearchStatus(
	callbacks: ToolCallbacks & { onMessageEnd: () => void },
	toolId: string,
	status: WebSearchStatus,
	details?: { errorCode?: string; query?: string; sources?: WebSearchSource[] }
) {
	const isLoading = status === 'searching'
	const failed = status === 'failed'
	const sources = details?.sources
	callbacks.onMessageEnd()
	callbacks.setToolStatus(`anthropic_web_search:${toolId}`, {
		content: failed
			? 'Web search failed'
			: isLoading
				? details?.query
					? `Searching the web for "${details.query}"...`
					: 'Searching the web...'
				: details?.query
					? `Searched the web for "${details.query}"`
					: 'Searched the web',
		error: failed
			? `Web search failed${details?.errorCode ? `: ${details.errorCode}` : ''}`
			: undefined,
		isLoading,
		isStreamingArguments: false,
		needsConfirmation: false,
		toolName: 'web_search',
		// Sources keep the card expanded (no auto-collapse) so the consulted
		// pages surface live as each search completes mid-stream.
		...(sources?.length
			? { webSearchSources: sources, showDetails: true, autoCollapseDetails: false }
			: {})
	})
}

// The query streams as partial JSON ({"query": "..."} cut mid-string), so
// JSON.parse fails until the block completes; regex out the string prefix to
// label the card while the model is still typing the query.
export function partialWebSearchQuery(partialJson: string): string | undefined {
	const m = partialJson.match(/"query"\s*:\s*"((?:[^"\\]|\\.)*)/)
	if (!m || !m[1]) return undefined
	try {
		return JSON.parse(`"${m[1]}"`)
	} catch {
		return undefined
	}
}

function anthropicWebSearchSources(
	content: Anthropic.Messages.WebSearchToolResultBlock['content']
): { errorCode?: string; sources?: WebSearchSource[] } {
	if (!Array.isArray(content)) {
		return { errorCode: content?.error_code }
	}
	return {
		sources: content
			.filter((r) => r.type === 'web_search_result')
			.map((r) => ({ url: r.url, title: r.title }))
	}
}

export async function getAnthropicCompletion(
	messages: ChatCompletionMessageParam[],
	abortController: AbortController,
	tools?: OpenAI.Chat.Completions.ChatCompletionFunctionTool[],
	options?: {
		forceModelProvider?: AIProviderModel
		anthropicClient?: Anthropic
		webSearch?: boolean
		reasoningEffort?: string
	}
): Promise<MessageStream> {
	const { provider, config } = getProviderAndCompletionConfig({
		messages,
		stream: true,
		forceModelProvider: options?.forceModelProvider
	})
	const { system, messages: anthropicMessages } = convertOpenAIToAnthropicMessages(messages)
	let anthropicTools = convertOpenAIToolsToAnthropic(tools)

	// Enable Anthropic's server-side web search tool. The proxy forwards the body
	// verbatim, so this reaches Anthropic as a native server tool that it executes
	// itself (no client round-trip).
	if (options?.webSearch) {
		anthropicTools = [
			...(anthropicTools ?? []),
			{ type: 'web_search_20250305', name: 'web_search', max_uses: 5 }
		]
	}

	const client = options?.anthropicClient ?? workspaceAIClients.getAnthropicClient()

	// Adds output_config.effort + adaptive thinking when an effort is set;
	// no-op otherwise. Returns the base shape unchanged when off.
	const anthropicParams = applyReasoningToConfig(
		{
			model: config.model,
			max_tokens: config.max_tokens as number,
			messages: anthropicMessages,
			...(system && { system }),
			...(anthropicTools && { tools: anthropicTools })
		},
		'anthropic',
		options?.reasoningEffort
	)

	const stream = client.messages.stream(anthropicParams, {
		signal: abortController.signal,
		headers: {
			'X-Provider': provider,
			'anthropic-version': '2023-06-01',
			'X-Anthropic-SDK': 'true'
		}
	})

	return stream
}

export async function parseAnthropicCompletion(
	completion: MessageStream,
	callbacks: ToolCallbacks & {
		onNewToken: (token: string) => void
		onMessageEnd: () => void
	},
	messages: ChatCompletionMessageParam[],
	addedMessages: ChatCompletionMessageParam[],
	tools: Tool<any>[],
	helpers: any,
	abortController?: AbortController,
	options?: { workspace?: string }
): Promise<ParsedCompletionResult> {
	let toolCallsToProcess: ChatCompletionMessageFunctionToolCall[] = []
	let error = null

	let currentStreamingTool:
		| { tempId: string; shouldStream: boolean; toolName: string; isWebSearch?: boolean }
		| undefined = undefined
	let accumulatedJson = ''
	// server_tool_use id → query, filled while the query streams; the paired
	// web_search_tool_result block only carries tool_use_id.
	const webSearchQueries = new Map<string, string>()
	// Result blocks already surfaced from stream events: the end-of-turn message
	// handler must not re-emit these — re-setting the status would re-expand a
	// card the user collapsed in the meantime.
	const surfacedWebSearchResults = new Set<string>()

	completion.on('streamEvent', (event: RawMessageStreamEvent) => {
		if (event.type === 'content_block_start') {
			const block = event.content_block
			if (block.type === 'tool_use') {
				const toolName = block.name
				const toolId = block.id as string

				const tool = tools.find((t) => t.def.function.name === toolName)
				const shouldStream = tool?.streamArguments ?? false

				callbacks.onMessageEnd()

				// Reset accumulated JSON for new tool
				accumulatedJson = ''
				currentStreamingTool = { tempId: toolId, shouldStream, toolName }

				callbacks.setToolStatus(toolId, {
					isLoading: true,
					content: tool?.streamingLabel ?? `Calling ${toolName}...`,
					toolName,
					isStreamingArguments: shouldStream,
					showFade: tool?.showFade,
					showDetails: tool?.showDetails,
					autoCollapseDetails: tool?.autoCollapseDetails
				})
			} else if (block.type === 'server_tool_use' && block.name === 'web_search') {
				accumulatedJson = ''
				currentStreamingTool = {
					tempId: block.id,
					shouldStream: false,
					toolName: 'web_search',
					isWebSearch: true
				}
				setAnthropicWebSearchStatus(callbacks, block.id, 'searching')
			} else if (block.type === 'web_search_tool_result') {
				// Server tool results arrive complete in content_block_start; surface
				// the source list now, while the model is still streaming the rest of
				// its turn, instead of waiting for the end-of-turn message event.
				surfacedWebSearchResults.add(block.tool_use_id)
				const { errorCode, sources } = anthropicWebSearchSources(block.content)
				setAnthropicWebSearchStatus(
					callbacks,
					block.tool_use_id,
					errorCode ? 'failed' : 'completed',
					{ errorCode, query: webSearchQueries.get(block.tool_use_id), sources }
				)
			}
		} else if (event.type === 'content_block_stop' && currentStreamingTool) {
			// Args fully streamed: the call is queued, not executing — tool calls run
			// sequentially after the message completes, and processToolCall flips each
			// back to loading when its turn starts. Without this, every call in a
			// multi-tool message spins while only the first is actually running.
			callbacks.setToolStatus(currentStreamingTool.tempId, {
				isLoading: false,
				isQueued: true,
				isStreamingArguments: false
			})
			currentStreamingTool = undefined
		}
	})

	// Stream summarized thinking deltas to the chat's collapsible reasoning block.
	completion.on('streamEvent', (event: RawMessageStreamEvent) => {
		if (event.type === 'content_block_start') {
			const block = event.content_block as any
			if (block?.type === 'thinking' || block?.type === 'redacted_thinking') {
				callbacks.onReasoningStart?.()
			}
		} else if (event.type === 'content_block_delta') {
			const delta = event.delta as any
			if (delta?.type === 'thinking_delta' && typeof delta.thinking === 'string') {
				callbacks.onReasoningDelta?.(delta.thinking)
			}
		}
	})

	completion.on('inputJson', (partialJson: string) => {
		if (currentStreamingTool?.isWebSearch) {
			accumulatedJson += partialJson
			const query = partialWebSearchQuery(accumulatedJson)
			if (query) {
				webSearchQueries.set(currentStreamingTool.tempId, query)
				setAnthropicWebSearchStatus(callbacks, currentStreamingTool.tempId, 'searching', { query })
			}
			return
		}
		if (currentStreamingTool?.shouldStream && currentStreamingTool.tempId) {
			// Accumulate the partial JSON
			accumulatedJson += partialJson

			// Try to parse and display
			try {
				const parsed = JSON.parse(accumulatedJson)
				callbacks.setToolStatus(currentStreamingTool.tempId, {
					parameters: parsed,
					isStreamingArguments: true,
					isLoading: true
				})
			} catch {
				// JSON incomplete, display as raw string
				callbacks.setToolStatus(currentStreamingTool.tempId, {
					parameters: accumulatedJson,
					isStreamingArguments: true,
					isLoading: true
				})
			}
		}
	})

	// Handle text streaming
	completion.on('text', (textDelta: string, _textSnapshot: string) => {
		callbacks.onNewToken(textDelta)
	})

	completion.on('message', (message: Message) => {
		// Final message blocks carry the complete query; overwrite whatever the
		// partial-JSON extraction reconstructed during streaming.
		for (const block of message.content) {
			if (block.type === 'server_tool_use' && block.name === 'web_search') {
				const query = (block.input as any)?.query
				if (typeof query === 'string' && query.length > 0) {
					webSearchQueries.set(block.id, query)
				}
			}
		}
		for (const block of message.content) {
			if (block.type === 'text') {
				const text = block.text
				const assistantMessage = { role: 'assistant' as const, content: text }
				messages.push(assistantMessage)
				addedMessages.push(assistantMessage)
				callbacks.onMessageEnd()
			} else if (
				block.type === 'web_search_tool_result' &&
				!surfacedWebSearchResults.has(block.tool_use_id)
			) {
				// Fallback for a result whose content_block_start was missed.
				const { errorCode, sources } = anthropicWebSearchSources(block.content)
				setAnthropicWebSearchStatus(
					callbacks,
					block.tool_use_id,
					errorCode ? 'failed' : 'completed',
					{ errorCode, query: webSearchQueries.get(block.tool_use_id), sources }
				)
			} else if (block.type === 'tool_use') {
				// Convert Anthropic tool calls to OpenAI format for compatibility
				toolCallsToProcess.push({
					id: block.id,
					type: 'function' as const,
					function: {
						name: block.name,
						arguments: JSON.stringify(block.input)
					}
				})
				// Preprocess tool if it has a preAction
				const tool = tools.find((t) => t.def.function.name === block.name)
				if (tool && tool.preAction) {
					tool.preAction({ toolCallbacks: callbacks, toolId: block.id })
				}
			}
		}

		// Clear temp tracking after processing
		currentStreamingTool = undefined
	})

	// Handle abort
	completion.on('abort', (e: any) => {
		// Check the AbortController's signal for the reason
		const abortReason = abortController?.signal.reason
		console.warn('Anthropic stream aborted:', {
			name: e?.name,
			message: e?.message,
			abortReason,
			wasAbortedByUser: abortReason === 'user_cancelled',
			signalAborted: abortController?.signal.aborted,
			cause: e?.cause,
			stack: e?.stack
		})
		error = e
	})

	// Handle errors
	completion.on('error', (e: any) => {
		console.error('Anthropic stream error:', {
			name: e?.name,
			message: e?.message,
			status: e?.status,
			headers: e?.headers,
			error: e?.error,
			cause: e?.cause,
			stack: e?.stack
		})
		error = e
	})

	// Wait for completion
	await completion.done()

	callbacks.onMessageEnd()

	if (error) {
		throw error
	}

	const finalMessage = await completion.finalMessage()
	const tokenUsage = anthropicUsageToChatTokenUsage(finalMessage.usage)

	// Process tool calls if any
	if (toolCallsToProcess.length > 0) {
		const assistantWithTools: ChatCompletionMessageParam = {
			role: 'assistant',
			tool_calls: toolCallsToProcess
		}
		// Preserve the assistant turn verbatim (thinking/redacted_thinking with their
		// signatures, server_tool_use + web_search_tool_result, text and tool_use) in
		// original order. Anthropic binds each thinking block's signature to the blocks
		// that precede it in the latest assistant message, so when this turn is replayed
		// to continue past its tool call it must be byte-identical: reordering thinking to
		// the front or dropping the web-search blocks invalidates a later block's
		// signature and the request 400s with "thinking blocks ... cannot be modified".
		// convertOpenAIToAnthropicMessages replays this content as-is.
		;(assistantWithTools as any)._anthropicContent = finalMessage.content
		messages.push(assistantWithTools)
		addedMessages.push(assistantWithTools)

		// Process each tool call
		for (const toolCall of toolCallsToProcess) {
			const messageToAdd = await processToolCall({
				tools,
				toolCall,
				helpers,
				toolCallbacks: callbacks,
				workspace: options?.workspace
			})
			messages.push(messageToAdd)
			addedMessages.push(messageToAdd)
		}
		appendPendingToolImages(messages, addedMessages, callbacks)
		return { shouldContinue: true, tokenUsage }
	}

	return { shouldContinue: false, tokenUsage }
}

export function convertOpenAIToAnthropicMessages(messages: ChatCompletionMessageParam[]): {
	system: TextBlockParam[] | undefined
	messages: MessageParam[]
} {
	let system: TextBlockParam[] | undefined
	const anthropicMessages: MessageParam[] = []

	// A streamed assistant turn that ends in tool calls is persisted as one or more
	// standalone text messages followed by the tool-call message carrying
	// _anthropicContent. That text is already inside _anthropicContent (replayed verbatim
	// below), so drop the standalone copies — otherwise the text is duplicated and
	// emitted ahead of the turn's thinking blocks. The scan stops at the preceding
	// user/tool message, so only the current turn's own text is skipped.
	const skipStandaloneText = new Set<number>()
	for (let i = 0; i < messages.length; i++) {
		if (!(messages[i] as any)._anthropicContent) continue
		for (let j = i - 1; j >= 0; j--) {
			const m = messages[j]
			if (
				m.role === 'assistant' &&
				typeof m.content === 'string' &&
				!m.tool_calls &&
				!(m as any)._anthropicContent
			) {
				skipStandaloneText.add(j)
			} else {
				break
			}
		}
	}

	for (let i = 0; i < messages.length; i++) {
		const message = messages[i]
		if (skipStandaloneText.has(i)) continue
		if (message.role === 'system') {
			const systemText =
				typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
			// Convert system to array format with cache_control for caching
			system = [
				{
					type: 'text',
					text: systemText,
					cache_control: { type: 'ephemeral' }
				}
			]
			continue
		}

		if (message.role === 'user') {
			anthropicMessages.push({
				role: 'user',
				content: openAIUserContentToAnthropic(message.content)
			})
		} else if (message.role === 'assistant') {
			// Replay a captured assistant turn verbatim so its thinking-block signatures
			// stay valid (see the _anthropicContent note where the streamed turn is stored).
			const anthropicContent = (message as any)._anthropicContent
			if (Array.isArray(anthropicContent) && anthropicContent.length > 0) {
				anthropicMessages.push({ role: 'assistant', content: anthropicContent as any })
				continue
			}

			const content: any[] = []

			// Fallback for sessions persisted before _anthropicContent existed: re-inject
			// the preserved thinking blocks first (Anthropic requires thinking to precede
			// tool_use in the same assistant turn when thinking is enabled).
			const thinkingBlocks = (message as any)._anthropicThinkingBlocks
			if (Array.isArray(thinkingBlocks) && thinkingBlocks.length > 0) {
				content.push(...thinkingBlocks)
			}

			if (message.content) {
				content.push({
					type: 'text',
					text:
						typeof message.content === 'string' ? message.content : JSON.stringify(message.content)
				})
			}

			if (message.tool_calls) {
				for (const toolCall of message.tool_calls) {
					if (toolCall.type !== 'function') continue
					let input = {}
					try {
						input = JSON.parse(toolCall.function.arguments || '{}')
					} catch (e) {
						console.error('Failed to parse tool call arguments', e)
					}
					content.push({
						type: 'tool_use',
						id: toolCall.id,
						name: toolCall.function.name,
						input
					})
				}
			}

			if (content.length > 0) {
				anthropicMessages.push({
					role: 'assistant',
					content: content.length === 1 && content[0].type === 'text' ? content[0].text : content
				})
			}
		} else if (message.role === 'tool') {
			// Tool results must be in user messages in Anthropic format
			anthropicMessages.push({
				role: 'user',
				content: [
					{
						type: 'tool_result',
						tool_use_id: message.tool_call_id,
						content:
							typeof message.content === 'string'
								? message.content
								: JSON.stringify(message.content)
					}
				]
			})
		}
	}

	// Cache the conversation prefix: put an ephemeral breakpoint on the last content
	// block of the last message. Each continuation only appends a tool result plus the
	// next turn, so everything up to here is read from cache — which is what keeps
	// replaying assistant turns verbatim (web-search results included) affordable.
	// cache_control is valid on text/tool_use/tool_result/image blocks, but a thinking
	// or redacted_thinking block must never be modified, so skip the breakpoint there.
	if (anthropicMessages.length > 0) {
		const lastMessage = anthropicMessages[anthropicMessages.length - 1]
		if (typeof lastMessage.content === 'string') {
			lastMessage.content = [
				{ type: 'text', text: lastMessage.content, cache_control: { type: 'ephemeral' } }
			]
		} else if (Array.isArray(lastMessage.content) && lastMessage.content.length > 0) {
			const lastIndex = lastMessage.content.length - 1
			const lastBlock = lastMessage.content[lastIndex] as any
			if (lastBlock.type !== 'thinking' && lastBlock.type !== 'redacted_thinking') {
				// Clone the block instead of mutating in place: the array may be a verbatim
				// _anthropicContent turn that must stay unaltered for later requests.
				lastMessage.content = [
					...lastMessage.content.slice(0, lastIndex),
					{ ...lastBlock, cache_control: { type: 'ephemeral' } }
				]
			}
		}
	}

	return { system, messages: anthropicMessages }
}

export function convertOpenAIToolsToAnthropic(
	tools?: OpenAI.Chat.Completions.ChatCompletionFunctionTool[]
): ToolUnion[] | undefined {
	if (!tools || tools.length === 0) return undefined

	const anthropicTools: ToolUnion[] = tools.map((tool) => ({
		name: tool.function.name,
		description: tool.function.description,
		input_schema: (tool.function.parameters || {
			type: 'object',
			properties: {}
		}) as AnthropicTool.InputSchema
	}))

	// Add cache_control to the last tool to cache all tool definitions
	if (anthropicTools.length > 0) {
		anthropicTools[anthropicTools.length - 1].cache_control = { type: 'ephemeral' }
	}

	return anthropicTools
}
