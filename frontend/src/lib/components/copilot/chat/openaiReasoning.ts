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
	toolCalls
}: {
	content: string
	reasoning: ReasoningContentState
	toolCalls: ChatCompletionMessageFunctionToolCall[]
}): ChatCompletionAssistantMessageWithReasoning {
	return {
		role: 'assistant',
		...(content || reasoning.hasReasoningContent ? { content } : {}),
		...(reasoning.hasReasoningContent ? { reasoning_content: reasoning.reasoningContent } : {}),
		tool_calls: toolCalls
	}
}
