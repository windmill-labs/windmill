import type { PricedTokens } from '../modelPricing'

export interface ChatTokenUsage {
	prompt: number
	completion: number
	total: number
	/**
	 * Subsets of `prompt`, split out because they are billed at different rates
	 * (a cached read is a fraction of an uncached one). `prompt` stays the whole
	 * input so the context gauge keeps measuring the whole request; uncached
	 * input is `prompt - cacheRead - cacheWrite`.
	 */
	cacheRead: number
	cacheWrite: number
	/** Cost in USD as billed, for the providers that report one. */
	cost?: number
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
	return { prompt: 0, completion: 0, total: 0, cacheRead: 0, cacheWrite: 0 }
}

export function addChatTokenUsage(
	total: ChatTokenUsage,
	usage: ChatTokenUsage | null | undefined
): ChatTokenUsage {
	if (!usage) {
		return total
	}

	const cost =
		total.cost === undefined && usage.cost === undefined
			? undefined
			: (total.cost ?? 0) + (usage.cost ?? 0)

	return {
		prompt: total.prompt + usage.prompt,
		completion: total.completion + usage.completion,
		total: total.total + usage.total,
		// `?? 0`: the cache split is newer than the field it lives on, so a usage
		// object read back from storage may predate it.
		cacheRead: (total.cacheRead ?? 0) + (usage.cacheRead ?? 0),
		cacheWrite: (total.cacheWrite ?? 0) + (usage.cacheWrite ?? 0),
		...(cost === undefined ? {} : { cost })
	}
}

/** Compact token count for readouts and tables (`1.2M`, `34k`, `567`). */
export function formatTokenCount(tokens: number): string {
	if (tokens >= 1_000_000) {
		return `${(tokens / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
	}
	if (tokens >= 1000) {
		return `${Math.round(tokens / 1000)}k`
	}
	return `${tokens}`
}

/**
 * Split a usage report into the four separately-billed token classes. `prompt`
 * counts the whole input, so the uncached share is whatever the cached classes
 * do not account for — which holds for both provider conventions below
 * (Anthropic adds its cache counts into `prompt`, OpenAI's already includes them).
 */
export function billedTokens(usage: ChatTokenUsage): PricedTokens {
	const cacheRead = usage.cacheRead ?? 0
	const cacheWrite = usage.cacheWrite ?? 0
	return {
		input: Math.max(0, usage.prompt - cacheRead - cacheWrite),
		cacheRead,
		cacheWrite,
		output: usage.completion
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
	const cacheWrite = usage?.cache_creation_input_tokens ?? 0
	const cacheRead = usage?.cache_read_input_tokens ?? 0
	const prompt = (usage?.input_tokens ?? 0) + cacheWrite + cacheRead
	const completion = usage?.output_tokens ?? 0

	return {
		prompt,
		completion,
		total: prompt + completion,
		cacheRead,
		cacheWrite
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
		total: usage?.total_tokens ?? prompt + completion,
		cacheRead: usage?.input_tokens_details?.cached_tokens ?? 0,
		// Automatic caching: nothing is billed for populating it, and no usage
		// field reports it either.
		cacheWrite: 0
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
				prompt_tokens_details?: {
					cached_tokens?: number | null
					/** Cache creation, reported by the providers that bill for it: OpenRouter
					 * passes Anthropic's through, and the Bedrock proxy folds
					 * `cacheWriteInputTokens` in here. OpenAI, whose caching is automatic and
					 * unbilled, reports no such field. */
					cache_write_tokens?: number | null
				} | null
				/** OpenRouter reports what it actually charged when the request opts in. */
				cost?: number | null
		  }
		| null
		| undefined
): ChatTokenUsage {
	const prompt = usage?.prompt_tokens ?? 0
	const completion = usage?.completion_tokens ?? 0

	return {
		prompt,
		completion,
		total: usage?.total_tokens ?? prompt + completion,
		cacheRead: usage?.prompt_tokens_details?.cached_tokens ?? 0,
		cacheWrite: usage?.prompt_tokens_details?.cache_write_tokens ?? 0,
		...(typeof usage?.cost === 'number' ? { cost: usage.cost } : {})
	}
}
