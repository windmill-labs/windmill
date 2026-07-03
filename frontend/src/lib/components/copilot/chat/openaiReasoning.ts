import type {
	ChatCompletionChunk,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'

type ChatCompletionDeltaWithReasoning = ChatCompletionChunk.Choice.Delta & {
	reasoning_content?: string | null
}

type ChatCompletionAssistantMessageWithReasoning = ChatCompletionMessageParam & {
	role: 'assistant'
	reasoning_content?: string
}

export type ReasoningContentState = {
	hasReasoningContent: boolean
	reasoningContent: string
}

export function getReasoningContentDelta(
	delta: ChatCompletionChunk.Choice.Delta
): string | null | undefined {
	return (delta as ChatCompletionDeltaWithReasoning).reasoning_content
}

/**
 * Mistral streams reasoning as structured content parts instead of a
 * `reasoning_content` field: `delta.content` is an array of
 * `{type: 'thinking', thinking: [{type: 'text', text}]}` chunks while the
 * model thinks, then plain strings for the answer. Split a content delta
 * into its reasoning and answer text (a plain-string delta is all answer).
 */
export function splitContentDelta(content: unknown): { reasoning: string; text: string } {
	if (typeof content === 'string') {
		return { reasoning: '', text: content }
	}
	if (!Array.isArray(content)) {
		return { reasoning: '', text: '' }
	}
	let reasoning = ''
	let text = ''
	for (const part of content) {
		if (part?.type === 'thinking' && Array.isArray(part.thinking)) {
			for (const t of part.thinking) {
				if (t?.type === 'text' && typeof t.text === 'string') {
					reasoning += t.text
				}
			}
		} else if (part?.type === 'text' && typeof part.text === 'string') {
			text += part.text
		}
	}
	return { reasoning, text }
}

/**
 * Whether the provider tolerates `reasoning_content` on input messages.
 * DeepSeek wants the field replayed in tool-call loops and pass-through
 * gateways ignore it, but Mistral hard-422s (`extra_forbidden`).
 */
function acceptsReasoningEcho(provider?: string): boolean {
	return provider !== 'mistral'
}

export function buildAssistantTextMessage(
	content: string
): ChatCompletionAssistantMessageWithReasoning {
	return {
		role: 'assistant',
		content
	}
}

export function buildAssistantToolCallMessage({
	content,
	reasoning,
	toolCalls,
	provider
}: {
	content: string
	reasoning: ReasoningContentState
	toolCalls: ChatCompletionMessageFunctionToolCall[]
	provider?: string
}): ChatCompletionAssistantMessageWithReasoning {
	const echoReasoning = reasoning.hasReasoningContent && acceptsReasoningEcho(provider)
	return {
		role: 'assistant',
		...(content || reasoning.hasReasoningContent ? { content } : {}),
		...(echoReasoning ? { reasoning_content: reasoning.reasoningContent } : {}),
		tool_calls: toolCalls
	}
}
