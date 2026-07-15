import type { AIProvider } from '$lib/gen'

// Azure AI Foundry fronts multiple model families under one resource. Claude
// deployments are served only through the Anthropic Messages API, so the chat must
// route them like the native Anthropic provider (Anthropic SDK, message format)
// rather than the OpenAI-compatible surface used for the rest of Foundry's catalog.
// Mirrors the backend `AIProvider::is_anthropic_model`.
export function usesAnthropicMessagesApi(provider: AIProvider, model: string): boolean {
	return (
		provider === 'anthropic' ||
		(provider === 'azure_foundry' && model.toLowerCase().startsWith('claude'))
	)
}

// gpt-5+ and o-series reasoning models reject the legacy `max_tokens` field on
// the OpenAI/Azure Chat Completions API and require `max_completion_tokens`
// instead. The check strips any provider prefix (e.g. OpenRouter's "openai/o3")
// so it matches the bare model id, and the o-series match requires a digit after
// the "o" (o1/o3/o4-mini) so it does not catch unrelated ids like Mistral's
// "open-mistral-*" or "optimus-*".
export function requiresMaxCompletionTokens(model: string) {
	const normalizedModel = model.toLowerCase()
	const baseModel = normalizedModel.split('/').pop() ?? normalizedModel
	return baseModel.startsWith('gpt-5') || /^o\d/.test(baseModel)
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

/**
 * Best-effort check that a model can accept image input. There is no per-model vision
 * metadata in the codebase, so this is deliberately permissive: it returns true unless
 * the model is a known text-only one that would 400 on an image part. Used to gate the
 * image-attach affordance and the screenshot follow-up; when unsure it allows the image
 * (the user explicitly attached it — better to try than to silently drop it).
 */
export function modelSupportsVision(
	provider: AIProvider | undefined,
	model: string | undefined
): boolean {
	if (!provider) return true
	const m = (model ?? '').toLowerCase()
	// Phi-4 is text-only, but Phi-4-multimodal is not, so it can't be matched by
	// name alone like the rest.
	if (m.includes('phi-4')) return m.includes('multimodal')
	return !TEXT_ONLY_MODELS.some((name) => m.includes(name))
}

/**
 * There is no per-model capability metadata, so this names the text-only models
 * Windmill offers as defaults (`defaultModels` above) and stays permissive for
 * anything custom: a wrong "no" blocks a working model, while a wrong "yes" only
 * costs one rejected turn. Keep it in step with the bundled defaults.
 *
 * Deliberately narrow: Llama 3.2's 11b/90b are vision models while its 1b/3b are
 * not, so the family can't be matched wholesale.
 */
const TEXT_ONLY_MODELS = [
	'codestral',
	'mistral-large',
	'deepseek-chat',
	'deepseek-reasoner',
	'deepseek-v3',
	'deepseek-r1',
	'o1-mini',
	'o3-mini',
	'embed',
	'llama-3.1',
	'llama-3.2-1b',
	'llama-3.2-3b',
	'llama-3.3'
]
