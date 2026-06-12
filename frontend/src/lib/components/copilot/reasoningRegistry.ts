import type { AIProvider, AIProviderModel } from '$lib/gen'

/**
 * Reasoning effort is provider/model-specific. We never normalize a single
 * cross-provider semantic — `'high'` for Claude means Claude's high, `'high'`
 * for OpenAI means OpenAI's high. The value is an open string (provider-native
 * token) so custom models and future levels are never blocked. A per-provider
 * registry supplies the *suggested* options for the UI and decides which models
 * are reasoning-capable; the actual stored/sent value is just a string.
 */
export type ReasoningEffort = string

/** Client-side extension of the selected model with a user-chosen effort. */
export type ReasoningProviderModel = AIProviderModel & { reasoning?: ReasoningEffort }

/** Sentinel meaning "user explicitly turned reasoning off" (distinct from unset). */
export const REASONING_OFF = 'off'

/** Default effort applied to reasoning-capable models when the user hasn't chosen. */
export const DEFAULT_REASONING_EFFORT = 'high'

/** Legacy model-string suffix that used to toggle Anthropic extended thinking. */
export const LEGACY_THINKING_SUFFIX = '/thinking'

/**
 * Strip the deprecated `/thinking` suffix from a model id. Old selections become
 * the plain model and pick up the default effort via the resolver below.
 */
export function stripLegacyThinkingSuffix(model: string): string {
	return model.endsWith(LEGACY_THINKING_SUFFIX)
		? model.slice(0, -LEGACY_THINKING_SUFFIX.length)
		: model
}

/** Bare model id without any provider/gateway prefix (e.g. OpenRouter's `openai/o3`). */
function baseModelId(model: string): string {
	const normalized = model.toLowerCase()
	return normalized.split('/').pop() ?? normalized
}

/**
 * Suggested effort levels per provider, sourced from each provider SDK's own
 * vocabulary. These are the static fallback; dynamic enrichment (Anthropic,
 * OpenRouter) can override with exact per-model levels.
 */
const PROVIDER_REASONING_LEVELS: Partial<Record<AIProvider, ReasoningEffort[]>> = {
	openai: ['minimal', 'low', 'medium', 'high'],
	azure_openai: ['minimal', 'low', 'medium', 'high'],
	openrouter: ['low', 'medium', 'high'],
	googleai: ['low', 'medium', 'high']
}

/**
 * Anthropic's effort ladder is model-dependent: `xhigh` exists only on Opus 4.7/4.8
 * and Fable; `max` on Opus 4.6+ and Sonnet 4.6. Offering an unsupported level would
 * 400, so scope the list to the model.
 */
function anthropicReasoningLevels(model: string): ReasoningEffort[] {
	const m = model.toLowerCase()
	if (/claude-opus-4-(7|8)/.test(m) || m.includes('fable')) {
		return ['low', 'medium', 'high', 'xhigh', 'max']
	}
	return ['low', 'medium', 'high', 'max']
}

/**
 * Conservative static predicate for whether a model accepts an effort knob.
 * Kept tight to avoid 400s on models that reject reasoning params; dynamic
 * enrichment widens it where the provider API exposes capability.
 */
function supportsReasoningStatic(provider: AIProvider, model: string): boolean {
	const m = model.toLowerCase()
	const base = baseModelId(model)
	switch (provider) {
		case 'anthropic':
			return /claude-opus-4-(5|6|7|8)/.test(m) || /claude-sonnet-4-6/.test(m) || m.includes('fable')
		case 'openai':
		case 'azure_openai':
			return base.startsWith('gpt-5') || /^o\d/.test(base)
		case 'openrouter':
			// best-effort markers; dynamic enrichment (supported_parameters) refines this
			return (
				base.startsWith('gpt-5') ||
				/^o\d/.test(base) ||
				/claude-(opus|sonnet)-4/.test(m) ||
				/gemini-(2\.5|3)/.test(m) ||
				m.includes('deepseek-r') ||
				m.includes('grok-4') ||
				m.includes(':thinking')
			)
		case 'googleai':
			return /gemini-(2\.5|3)/.test(m)
		default:
			return false
	}
}

export type ReasoningCapability = {
	supported: boolean
	/** Suggested levels for the UI control. Empty when unsupported. */
	levels: ReasoningEffort[]
}

/** Resolve the reasoning capability of a model from the static registry. */
export function getReasoningCapability(provider: AIProvider, model: string): ReasoningCapability {
	const bareModel = stripLegacyThinkingSuffix(model)
	const supported = supportsReasoningStatic(provider, bareModel)
	if (!supported) {
		return { supported: false, levels: [] }
	}
	const levels =
		provider === 'anthropic'
			? anthropicReasoningLevels(bareModel)
			: (PROVIDER_REASONING_LEVELS[provider] ?? ['low', 'medium', 'high'])
	return { supported, levels }
}

export function supportsReasoning(provider: AIProvider, model: string): boolean {
	return getReasoningCapability(provider, model).supported
}

/**
 * The effective effort to send for a given model selection:
 * - explicit `REASONING_OFF` (or empty) -> undefined (send nothing)
 * - explicit level -> that level
 * - unset + reasoning-capable -> DEFAULT_REASONING_EFFORT (default-on)
 * - unset + not capable -> undefined
 */
export function resolveEffectiveReasoning(
	modelProvider: ReasoningProviderModel
): ReasoningEffort | undefined {
	const reasoning = modelProvider.reasoning
	if (reasoning === REASONING_OFF || reasoning === '') {
		return undefined
	}
	if (reasoning) {
		return reasoning
	}
	return supportsReasoning(modelProvider.provider, modelProvider.model)
		? DEFAULT_REASONING_EFFORT
		: undefined
}

export type ReasoningApiKind = 'anthropic' | 'responses' | 'completions'

/**
 * Inject an effort level into a request config for the given completion path.
 * `effort` is resolved by the caller (see `resolveEffectiveReasoning`), so the
 * default-on logic stays in chat context and never leaks into background paths.
 * Returns the config unchanged when `effort` is falsy (byte-identical to the
 * pre-feature request, so the change is backward-safe when reasoning is off).
 *
 * Values are open strings (provider-native tokens) that don't match the SDKs'
 * narrow enums, and the native Anthropic params aren't typed for adaptive
 * thinking in the pinned SDK — so the additions are cast; the proxy forwards the
 * raw JSON body regardless.
 */
export function applyReasoningToConfig<T extends Record<string, any>>(
	config: T,
	apiKind: ReasoningApiKind,
	effort: ReasoningEffort | undefined
): T {
	if (!effort) {
		return config
	}
	switch (apiKind) {
		case 'anthropic': {
			// Adaptive thinking rejects sampling params; strip them when reasoning is on.
			const { temperature: _t, top_p: _p, top_k: _k, ...rest } = config as Record<string, any>
			return {
				...rest,
				output_config: { ...(config.output_config ?? {}), effort },
				// Adaptive thinking replaces budget_tokens; summarized display makes the
				// thinking stream renderable in the chat.
				thinking: { type: 'adaptive', display: 'summarized' }
			} as unknown as T
		}
		case 'responses':
			return {
				...config,
				reasoning: { ...((config as any).reasoning ?? {}), effort }
			} as unknown as T
		case 'completions':
			return {
				...config,
				reasoning_effort: effort
			} as unknown as T
	}
}
