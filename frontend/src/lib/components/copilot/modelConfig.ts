import type { AIProvider } from '$lib/gen'

export type ParsedModelId = {
	/** Vendor namespace when the id carries one (`anthropic/claude-sonnet-5`). */
	vendor: string | undefined
	/** Bare model id: no vendor prefix, no variant suffix. */
	base: string
}

/**
 * Split a model id into the parts the predicates below match on. Gateways decorate
 * the vendor's id in ways a raw substring check misses: OpenRouter marks its own
 * floating aliases with a `~` prefix (`~anthropic/claude-sonnet-latest`, distinct
 * from the vendor-pinned `anthropic/claude-sonnet-5`) and appends `:variant`
 * suffixes (`:free`, `:thinking`). Match on the parsed parts, never on the raw id.
 *
 * `base` is the last `/` segment: the deprecated `<model>/thinking` selection puts
 * its marker where a gateway puts the model, so callers that must resolve one of
 * those ids first run it through `stripLegacyThinkingSuffix`.
 */
export function parseModelId(model: string): ParsedModelId {
	const normalized = model.toLowerCase().replace(/^~/, '')
	const segments = normalized.split('/')
	const last = segments[segments.length - 1]
	const colon = last.indexOf(':')
	return {
		vendor: segments.length > 1 ? segments[0] : undefined,
		base: colon > 0 ? last.slice(0, colon) : last
	}
}

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

// Anthropic bills a cached prefix at a tenth of the input rate, but only creates one
// where an explicit `cache_control` breakpoint sits. With no breakpoint the whole
// prompt is charged in full on every iteration of a chat. The native Anthropic path
// sets its own breakpoints; OpenRouter forwards them over the OpenAI-compatible surface
// but only documents them for Anthropic-backed models, so the gate is on the routed
// model rather than the provider alone.
export function usesOpenRouterPromptCaching(provider: AIProvider, model: string): boolean {
	return provider === 'openrouter' && parseModelId(model).vendor === 'anthropic'
}

// gpt-5+ and o-series reasoning models reject the legacy `max_tokens` field on
// the OpenAI/Azure Chat Completions API and require `max_completion_tokens`
// instead. The check runs on the bare model id (so OpenRouter's "openai/o3"
// matches), and the o-series match requires a digit after the "o" (o1/o3/o4-mini)
// so it does not catch unrelated ids like Mistral's "open-mistral-*" or "optimus-*".
export function requiresMaxCompletionTokens(model: string) {
	const baseModel = parseModelId(model).base
	return baseModel.startsWith('gpt-5') || /^o\d/.test(baseModel)
}

// Context windows of the models we know, most specific entry first — the first
// name found in the bare model id wins, so vendor-namespaced and date-suffixed
// ids (anthropic.claude-sonnet-4-6-...-v1:0, gpt-5.2-2026-01-01) still resolve.
// Conservative family fallbacks sit below the explicit entries; models not
// listed at all resolve to undefined. Consumers that need a number regardless
// (trim/compaction, the usage indicator) go through getModelContextWindow,
// whose conservative 128K fallback keeps a limit enforced and is surfaced to
// the user as an assumed window.
const MODEL_CONTEXT_WINDOWS: [name: string, contextWindow: number][] = [
	// Anthropic — Sonnet/Opus 4.6+ ship a 1M window at standard pricing (GA);
	// Haiku, older Claude models (3.x, 4.0, 4.1, 4.5) and date-suffixed Claude 4
	// base ids (claude-sonnet-4-20250514) fall through to 200K
	['claude-fable-5', 1_000_000],
	['claude-mythos-5', 1_000_000],
	['claude-opus-5', 1_000_000],
	['claude-sonnet-5', 1_000_000],
	['claude-opus-4-8', 1_000_000],
	['claude-opus-4-7', 1_000_000],
	['claude-opus-4-6', 1_000_000],
	['claude-sonnet-4-6', 1_000_000],
	['claude', 200_000],
	// OpenAI — gpt-5 covers the base family (-mini / -nano) and the 5.1/5.2
	// revisions, all 400K; 5.4/5.5 moved to 1M and 5.6 to 1.05M
	['gpt-5.6', 1_050_000],
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
	// DeepSeek — the V4 family (pro / flash) is 1M. The deepseek-chat /
	// deepseek-reasoner aliases were retired 2026-07-24 but can still sit in a
	// saved selection, so they keep resolving to the window they had.
	['deepseek-v4', 1_000_000],
	['deepseek-chat', 1_000_000],
	['deepseek-reasoner', 1_000_000],
	['deepseek', 128_000],
	// Alibaba — Qwen3-Max is 256K. No qwen family fallback: variant windows range
	// from 8K (character models) to 1M, too wide for even a conservative guess
	['qwen3-max', 256_000],
	// Others — Mistral Medium 3.5 is 256K, reachable under both its version and
	// the `-latest` alias. There is deliberately no `mistral-medium` family row:
	// pinned older snapshots are 128K, and over-claiming a window overflows it.
	['mistral-medium-3.5', 256_000],
	['mistral-medium-latest', 256_000],
	['llama', 128_000],
	['codestral', 32_000]
]

// Version separators differ by route to the same model: Anthropic writes
// `claude-opus-4-8`, OpenRouter writes `anthropic/claude-opus-4.8`. Collapsing
// dots to dashes on both sides keeps one table entry covering every route —
// without it a dot-versioned id falls through to a coarser family entry.
function normalizeVersionSeparators(model: string): string {
	return model.replace(/\./g, '-')
}

/** Suffixes that name a route to a model rather than a different model. */
const DECORATIVE_SUFFIXES = ['latest', 'preview', 'beta', 'stable']

/**
 * Compile a most-specific-first `[name, value]` table into matchers against the
 * bare model id. Shared with the pricing table so both resolve the same set of
 * ids — a model whose window is known but whose price is not (or vice versa)
 * should be a gap in one table, never a difference in matching.
 *
 * An entry that ends on a version digit must not run into a longer version:
 * `gpt-4.1` collapses to `gpt-4-1`, which would otherwise claim
 * `gpt-4-1106-preview`. Suffixes that continue with a separator
 * (`claude-opus-4-8` in `...-4-8-v1`, `gpt-5` in `gpt-5-mini`) still match.
 * Family fallbacks ending on a letter get no such guard — a version welded
 * straight onto the name (`llama3.1`) is exactly what they exist to catch.
 */
export function buildModelMatchers<T>(
	entries: [name: string, value: T][],
	{ strictVariants = false }: { strictVariants?: boolean } = {}
): [RegExp, T][] {
	return entries.map(([name, value]) => {
		const pattern = normalizeVersionSeparators(name).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
		const guards = [
			// An entry ending on a version digit must not run into a longer version.
			/\d$/.test(pattern) ? '(?!\\d)' : '',
			// A named sub-model (`gpt-5-pro`, `gpt-5-mini`) is a different model with
			// its own price, not another route to this one — so under strictVariants an
			// entry does not match when a further *name* segment follows. What follows
			// is only a decoration when it is a date (`-20251101`), Bedrock's `-v1`, or
			// one of the alias words below, at the very end of the id
			// (`claude-3-5-haiku-latest` is the same model as `claude-3-5-haiku`, and is
			// a shipped default; `gpt-5-preview-pro` would be a different one again).
			// Off by default: for a context window an inherited value is a safe
			// approximation, for a price it is a wrong number.
			// A further revision segment (`gpt-5` vs `gpt-5-4-mini`) is a different model
			// too, and the entry-ends-on-a-digit guard above does not catch it once the
			// separator is normalized. Only a short segment: a date is digits as well
			// (`-20251101`) and stays a decoration.
			strictVariants ? '(?!-\\d{1,3}(?:$|-))' : '',
			strictVariants
				? `(?!-(?!(?:v\\d|${DECORATIVE_SUFFIXES.join('|')})$)[a-z])`
				: ''
		].join('')
		return [new RegExp(pattern + guards), value]
	})
}

/**
 * The `provider:model` key the workspace AI settings use for their per-model maps
 * (`max_tokens_per_model`, `model_pricing`). A bare model id is not enough: the
 * same id can be served by more than one provider at different rates.
 *
 * Matched exactly, unlike the fuzzy tables above. Those tables generalize across
 * every route to one model on purpose; a per-model *setting* must not, or an
 * admin could not give two variants of a family different values — and the key is
 * built from the exact id the provider config lists, which is the same string the
 * chat sends.
 */
export function modelKey(provider: AIProvider | string, model: string): string {
	return `${provider}:${model}`
}

export function matchModel<T>(matchers: [RegExp, T][], model: string): T | undefined {
	const id = normalizeVersionSeparators(parseModelId(model).base)
	return matchers.find(([matcher]) => matcher.test(id))?.[1]
}

const MODEL_CONTEXT_WINDOW_MATCHERS = buildModelMatchers(MODEL_CONTEXT_WINDOWS)

export function getKnownModelContextWindow(model: string): number | undefined {
	return matchModel(MODEL_CONTEXT_WINDOW_MATCHERS, model)
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
	return !TEXT_ONLY_MODELS.has(`${provider}:${(model ?? '').toLowerCase()}`)
}

/**
 * Models whose provider API refuses image content, matched by exact
 * `provider:model` pair — not by id alone, because an id proves nothing about a
 * different endpoint (a Custom AI deployment may serve a vision model under a
 * name that collides with someone's text-only id, and there is no override).
 *
 * The question is not whether a model can see, but whether its provider's API
 * accepts image parts — the two diverge, and the divergence is invisible from a
 * name: DeepSeek V4 ships vision in its chat UI while its API has no image
 * content type, and o3-mini gained vision in ChatGPT that the API never exposed.
 * So this is a cache of one provider's API surface at one moment, and it rots.
 * Wrong entries are asymmetric: a missing one costs a single turn and
 * self-corrects (the request fails, the image is dropped, the user is told),
 * while a wrong one blocks a working model with no override. Hence exact pairs
 * only, and only where a provider doc says so.
 *
 * Substrings are specifically avoided: `mistral-large` would also match
 * Mistral Large 3, which does take images, and `phi-4` would match
 * Phi-4-multimodal, which does too.
 */
const TEXT_ONLY_MODELS = new Set([
	'openai:o1-mini',
	'openai:o3-mini',
	'azure_openai:o1-mini',
	'azure_openai:o3-mini',
	'mistral:codestral-latest',
	// deepseek — vision exists in their chat product, not in the API
	'deepseek:deepseek-v4-pro',
	'deepseek:deepseek-v4-flash',
	'deepseek:deepseek-chat',
	'deepseek:deepseek-reasoner',
	'groq:llama-3.3-70b-versatile',
	'groq:llama-3.1-8b-instant',
	// gpt-oss (text-only everywhere it is hosted) — on groq it succeeds the two
	// llama entries above, which retire 2026-08-16
	'groq:openai/gpt-oss-120b',
	'groq:openai/gpt-oss-20b',
	'openrouter:openai/gpt-oss-120b',
	'openrouter:openai/gpt-oss-20b',
	'togetherai:openai/gpt-oss-120b',
	'togetherai:openai/gpt-oss-20b',
	// azure_foundry serves DeepSeek-V4-Pro under the same id as deepseek's API
	'azure_foundry:deepseek-v4-pro',
	'azure_foundry:deepseek-r1',
	'azure_foundry:llama-3.3-70b-instruct',
	'azure_foundry:phi-4',
	'azure_foundry:mistral-large-2411',
	'openrouter:meta-llama/llama-3.2-3b-instruct:free',
	'togetherai:meta-llama/llama-3.3-70b-instruct-turbo'
])
