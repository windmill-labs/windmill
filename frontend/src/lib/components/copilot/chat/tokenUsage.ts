export interface ChatTokenUsage {
	prompt: number
	completion: number
	total: number
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

export function anthropicUsageToChatTokenUsage(usage: {
	input_tokens?: number | null
	output_tokens?: number | null
	cache_creation_input_tokens?: number | null
	cache_read_input_tokens?: number | null
} | null | undefined): ChatTokenUsage {
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

export function openAIResponsesUsageToChatTokenUsage(usage: {
	input_tokens?: number | null
	output_tokens?: number | null
	total_tokens?: number | null
} | null | undefined): ChatTokenUsage {
	const prompt = usage?.input_tokens ?? 0
	const completion = usage?.output_tokens ?? 0

	return {
		prompt,
		completion,
		total: usage?.total_tokens ?? prompt + completion
	}
}

export function openAICompletionsUsageToChatTokenUsage(usage: {
	prompt_tokens?: number | null
	completion_tokens?: number | null
	total_tokens?: number | null
} | null | undefined): ChatTokenUsage {
	const prompt = usage?.prompt_tokens ?? 0
	const completion = usage?.completion_tokens ?? 0

	return {
		prompt,
		completion,
		total: usage?.total_tokens ?? prompt + completion
	}
}
