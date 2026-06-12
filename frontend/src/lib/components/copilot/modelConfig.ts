import type { AIProviderModel } from '$lib/gen'

export function modelDisallowsSamplingParams(model: string) {
	const normalizedModel = model.toLowerCase()
	// Strip any provider prefix (e.g. OpenRouter's "openai/o3") so the
	// reasoning-model check matches the bare model id rather than the prefix.
	const baseModel = normalizedModel.split('/').pop() ?? normalizedModel
	// gpt-5+ and o-series reasoning models reject sampling params such as
	// temperature (only the default value is supported), regardless of which
	// provider/gateway routes the request — so this must stay provider-agnostic.
	// The o-series match requires a digit after the "o" (o1/o3/o4-mini) so it
	// does not catch unrelated ids like Mistral's "open-mistral-*" or "optimus-*".
	return (
		normalizedModel.includes('claude-fable-5') ||
		normalizedModel.includes('claude-opus-4-7') ||
		normalizedModel.includes('claude-opus-4-8') ||
		baseModel.startsWith('gpt-5') ||
		/^o\d/.test(baseModel)
	)
}

export function getDefaultChatTemperature(modelProvider: AIProviderModel): number | undefined {
	if (modelDisallowsSamplingParams(modelProvider.model)) {
		return undefined
	}

	return 0
}

// Context windows of the models we know, most specific entry first — the first
// name included in the model id wins, so provider-prefixed and date-suffixed
// ids (anthropic.claude-sonnet-4-6-...-v1:0, gpt-5.2-2026-01-01) still resolve.
// Conservative family fallbacks sit below the explicit entries; models not
// listed at all resolve to undefined, which disables auto-trimming and the
// indicator denominator.
const MODEL_CONTEXT_WINDOWS: [name: string, contextWindow: number][] = [
	// Anthropic — Sonnet/Opus 4.6+ ship a 1M window at standard pricing (GA);
	// Haiku, older Claude models (3.x, 4.0, 4.1, 4.5) and date-suffixed Claude 4
	// base ids (claude-sonnet-4-20250514) fall through to 200K
	['claude-fable-5', 1_000_000],
	['claude-opus-4-8', 1_000_000],
	['claude-opus-4-7', 1_000_000],
	['claude-opus-4-6', 1_000_000],
	['claude-sonnet-4-6', 1_000_000],
	['claude', 200_000],
	// OpenAI — gpt-5 covers the base family (-mini / -nano) and the 5.1/5.2
	// revisions, all 400K; only 5.4+ moved to 1M
	['gpt-5.5', 1_000_000],
	['gpt-5.4', 1_000_000],
	['gpt-5', 400_000],
	['gpt-4.1', 1_000_000],
	['gpt-4o', 128_000],
	['o4-mini', 200_000],
	['o3', 200_000],
	// Google — the 2.5 / 3 / 3.1 Gemini families are all 1M
	['gemini-3.1', 1_000_000],
	['gemini-3', 1_000_000],
	['gemini-2.5', 1_000_000],
	// DeepSeek — the V4 family is 1M; deepseek-chat / deepseek-reasoner are
	// aliases of V4-Flash since April 2026
	['deepseek-v4', 1_000_000],
	['deepseek-chat', 1_000_000],
	['deepseek-reasoner', 1_000_000],
	['deepseek', 128_000],
	// Others
	['llama', 128_000],
	['codestral', 32_000]
]

export function getKnownModelContextWindow(model: string): number | undefined {
	return MODEL_CONTEXT_WINDOWS.find(([name]) => model.includes(name))?.[1]
}

export function getModelContextWindow(model: string) {
	// Trim/compaction logic needs a number; assume a conservative window when unknown.
	return getKnownModelContextWindow(model) ?? 128000
}
