export interface ChatTokenUsage {
	prompt: number
	completion: number
	total: number
}

/**
 * Context usage persisted by earlier versions, which anchored the provider
 * report to a message index and re-based it on system-prompt/tool changes.
 * Usage is now a plain token count; old chats loaded from IndexedDB are
 * collapsed to it via `normalizeContextUsage`.
 */
export interface LegacyContextTokenSnapshot {
	tokens: number
	atMessageIndex: number
	overheadEstimate?: number
}

export type PersistedContextUsage = number | LegacyContextTokenSnapshot

export function normalizeContextUsage(
	value: PersistedContextUsage | undefined
): number | undefined {
	if (value === undefined) {
		return undefined
	}
	return typeof value === 'number' ? value : value.tokens
}

export function emptyChatTokenUsage(): ChatTokenUsage {
	return { prompt: 0, completion: 0, total: 0 }
}

export function addChatTokenUsage(
	total: ChatTokenUsage,
	usage: ChatTokenUsage | null | undefined
): ChatTokenUsage {
	if (!usage) {
		return total
	}

	return {
		prompt: total.prompt + usage.prompt,
		completion: total.completion + usage.completion,
		total: total.total + usage.total
	}
}

export function anthropicUsageToChatTokenUsage(
	usage:
		| {
				input_tokens?: number | null
				output_tokens?: number | null
				cache_creation_input_tokens?: number | null
				cache_read_input_tokens?: number | null
		  }
		| null
		| undefined
): ChatTokenUsage {
	const prompt =
		(usage?.input_tokens ?? 0) +
		(usage?.cache_creation_input_tokens ?? 0) +
		(usage?.cache_read_input_tokens ?? 0)
	const completion = usage?.output_tokens ?? 0

	return {
		prompt,
		completion,
		total: prompt + completion
	}
}

// Unlike Anthropic, OpenAI's input_tokens already includes cached tokens
// (input_tokens_details.cached_tokens is a subset), so it must not be added again.
export function openAIResponsesUsageToChatTokenUsage(
	usage:
		| {
				input_tokens?: number | null
				output_tokens?: number | null
				total_tokens?: number | null
				input_tokens_details?: { cached_tokens?: number | null } | null
		  }
		| null
		| undefined
): ChatTokenUsage {
	const prompt = usage?.input_tokens ?? 0
	const completion = usage?.output_tokens ?? 0

	return {
		prompt,
		completion,
		total: usage?.total_tokens ?? prompt + completion
	}
}

// Unlike Anthropic, OpenAI's prompt_tokens already includes cached tokens
// (prompt_tokens_details.cached_tokens is a subset), so it must not be added again.
export function openAICompletionsUsageToChatTokenUsage(
	usage:
		| {
				prompt_tokens?: number | null
				completion_tokens?: number | null
				total_tokens?: number | null
				prompt_tokens_details?: { cached_tokens?: number | null } | null
		  }
		| null
		| undefined
): ChatTokenUsage {
	const prompt = usage?.prompt_tokens ?? 0
	const completion = usage?.completion_tokens ?? 0

	return {
		prompt,
		completion,
		total: usage?.total_tokens ?? prompt + completion
	}
}
