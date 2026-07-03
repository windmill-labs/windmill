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
 * vocabulary.
 */
const PROVIDER_REASONING_LEVELS: Partial<Record<AIProvider, ReasoningEffort[]>> = {
	// DeepSeek accepts the full five-token vocabulary but only two levels are
	// real: low/medium are server-mapped to high and xhigh to max — offering
	// them would be a no-op knob.
	deepseek: ['high', 'max'],
	// Mistral's only effort token besides the 'none' disable is 'high'
	// (anything else is rejected), so the knob is effectively on/off.
	mistral: ['high']
}

/**
 * OpenRouter validates effort against its own vocabulary
 * (minimal..xhigh + none) and translates per underlying provider, so the
 * real ladder depends on the model family: Anthropic gets all five as
 * distinct budget ratios (minimal 10% .. xhigh 95% of max_tokens); Gemini
 * maps to thinkingLevel with xhigh clamped to high (a no-op vs high);
 * OpenAI gets the token passed through verbatim, so the per-model OpenAI
 * scoping applies; DeepSeek server-maps low/medium to high and xhigh to max.
 */
function openrouterReasoningLevels(model: string): ReasoningEffort[] {
	const m = model.toLowerCase()
	const base = baseModelId(model)
	if (/claude-(opus|sonnet)-4/.test(m)) {
		return ['minimal', 'low', 'medium', 'high', 'xhigh']
	}
	if (m.includes('gemini-')) {
		return geminiReasoningLevels(m)
	}
	if (base.startsWith('gpt-5') || /^o\d/.test(base)) {
		return openaiReasoningLevels(base)
	}
	if (m.includes('deepseek-v4')) {
		return ['high', 'xhigh']
	}
	return ['low', 'medium', 'high']
}

/**
 * OpenAI's effort vocabulary is model-dependent: `minimal` exists on gpt-5 but
 * not on gpt-5.1+, `xhigh` only on gpt-5.5; o-series take low/medium/high.
 * An unsupported level is rejected, so scope the list to the model.
 */
function openaiReasoningLevels(model: string): ReasoningEffort[] {
	const base = baseModelId(model)
	if (/^gpt-5\.5/.test(base)) {
		return ['low', 'medium', 'high', 'xhigh']
	}
	if (/^gpt-5\./.test(base)) {
		return ['low', 'medium', 'high']
	}
	if (/^gpt-5/.test(base)) {
		return ['minimal', 'low', 'medium', 'high']
	}
	return ['low', 'medium', 'high']
}

/**
 * Gemini's level ladder is model-dependent: Gemini 3+ Flash / Flash-Lite accept
 * `minimal`, while 3.x Pro does not (and cannot disable thinking). Gemini 2.5
 * uses numeric budgets — the proxy maps the three tiers to budget values, so
 * `minimal` is not offered there.
 */
function geminiReasoningLevels(model: string): ReasoningEffort[] {
	const m = model.toLowerCase()
	const isGemini3Plus = !m.includes('gemini-2.5')
	if (isGemini3Plus && (m.includes('flash') || m.includes('lite'))) {
		return ['minimal', 'low', 'medium', 'high']
	}
	return ['low', 'medium', 'high']
}

/**
 * Gemini Pro models cannot turn thinking off — the API enforces a floor
 * (level `low` on 3.x Pro, a 128-token budget on 2.5 Pro), so an off option
 * would silently mean "lowest". Flash / Flash-Lite can truly disable
 * (budget 0 / level `minimal`).
 */
function geminiCanDisable(model: string): boolean {
	return !model.toLowerCase().includes('pro')
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
 * Kept tight to avoid 400s on models that reject reasoning params.
 */
function supportsReasoningStatic(provider: AIProvider, model: string): boolean {
	const m = model.toLowerCase()
	const base = baseModelId(model)
	switch (provider) {
		case 'anthropic':
		// Bedrock serves the same Claude models under prefixed ids
		// (e.g. `us.anthropic.claude-opus-4-6-v1`), so match on the full string.
		case 'aws_bedrock':
			// 4.6+ only: Opus 4.5 rejects adaptive thinking (and, on Bedrock,
			// the whole output_config surface) — live-verified hard 400.
			return /claude-opus-4-(6|7|8)/.test(m) || /claude-sonnet-4-6/.test(m) || m.includes('fable')
		case 'openai':
		case 'azure_openai':
		// Azure AI Foundry hosts OpenAI models too; the model-id gate keeps
		// this a no-op for its non-OpenAI catalog (Llama/DeepSeek/etc.).
		case 'azure_foundry':
			return base.startsWith('gpt-5') || /^o\d/.test(base)
		case 'openrouter':
			// Best-effort markers for models whose `supported_parameters` include
			// `reasoning` in OpenRouter's catalog; OpenRouter translates the effort
			// per underlying provider.
			return (
				base.startsWith('gpt-5') ||
				/^o\d/.test(base) ||
				/claude-(opus|sonnet)-4/.test(m) ||
				/gemini-(2\.5|3)/.test(m) ||
				m.includes('deepseek-r') ||
				m.includes('deepseek-v4') ||
				m.includes('grok-4') ||
				m.includes(':thinking')
			)
		case 'googleai':
			return /gemini-(2\.5|3)/.test(m)
		case 'deepseek':
			// All current API models take reasoning_effort (live-verified). The
			// deprecated `deepseek-chat` alias is excluded: its documented meaning
			// is "non-thinking mode", and sending an effort would silently flip it
			// into thinking mode — picking that alias is itself an off choice.
			return base.startsWith('deepseek') && base !== 'deepseek-chat'
		case 'mistral':
			// Only the ids verified to accept reasoning_effort; other models
			// (large, magistral, ministral, pinned versions) reject the param.
			return /^mistral-(small|medium)-latest$/.test(base) || base.startsWith('mistral-medium-3-5')
		default:
			return false
	}
}

export type ReasoningCapability = {
	supported: boolean
	/** Suggested levels for the UI control. Empty when unsupported. */
	levels: ReasoningEffort[]
	/**
	 * Whether the model can truly turn reasoning off. When false the UI must
	 * not offer an off option — the provider would coerce it to the lowest
	 * level, making the switch a lie.
	 */
	canDisable: boolean
}

/** Resolve the reasoning capability of a model from the static registry. */
export function getReasoningCapability(provider: AIProvider, model: string): ReasoningCapability {
	const bareModel = stripLegacyThinkingSuffix(model)
	const supported = supportsReasoningStatic(provider, bareModel)
	if (!supported) {
		return { supported: false, levels: [], canDisable: false }
	}
	const levels =
		provider === 'anthropic' || provider === 'aws_bedrock'
			? anthropicReasoningLevels(bareModel)
			: provider === 'googleai'
				? geminiReasoningLevels(bareModel)
				: provider === 'openai' || provider === 'azure_openai' || provider === 'azure_foundry'
					? openaiReasoningLevels(bareModel)
					: provider === 'openrouter'
						? openrouterReasoningLevels(bareModel)
						: (PROVIDER_REASONING_LEVELS[provider] ?? ['low', 'medium', 'high'])
	return { supported, levels, canDisable: canDisableReasoning(provider, bareModel) }
}

/**
 * Whether selecting "off" truly disables reasoning for the model. Off is
 * sent either as an explicit provider disable (see `explicitOffToken`) or by
 * omitting the effort — which only works where the model doesn't reason by
 * default.
 */
function canDisableReasoning(provider: AIProvider, model: string): boolean {
	const m = model.toLowerCase()
	const base = baseModelId(model)
	switch (provider) {
		case 'anthropic':
		case 'aws_bedrock':
			// Claude 4.6+ only think when asked, so omission is a real off —
			// except Fable, where thinking is always on (explicit disable 400s).
			return !m.includes('fable')
		case 'googleai':
			return geminiCanDisable(model)
		case 'openai':
		case 'azure_openai':
		case 'azure_foundry':
			// gpt-5.1+ accept effort 'none'; gpt-5 and o-series reject it and
			// reason at `medium` by default, so omission isn't off either.
			return /^gpt-5\./.test(base)
		case 'openrouter':
			// 'none' is in OpenRouter's vocabulary, but the gateway can't
			// disable a model whose upstream can't — scope off per underlying
			// family, like the levels.
			if (/claude-(opus|sonnet)-4/.test(m)) {
				return true
			}
			if (m.includes('gemini-')) {
				return geminiCanDisable(m)
			}
			if (base.startsWith('gpt-5') || /^o\d/.test(base)) {
				return /^gpt-5\./.test(base)
			}
			if (m.includes('deepseek-v4')) {
				return true
			}
			// grok-4, deepseek-r1 and :thinking variants reason unconditionally.
			return false
		default:
			return true
	}
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

/**
 * Sentinel sent for the deepseek off case. It never reaches the wire as an
 * effort: the 'deepseek' branch of `applyReasoningToConfig` translates it to
 * the provider's `thinking: {type: "disabled"}` param (`reasoning_effort:
 * "none"` is rejected by their API).
 */
export const DEEPSEEK_OFF_SENTINEL: ReasoningEffort = 'none'

/**
 * Disable token to forward when the user explicitly turns reasoning off on a
 * model that reasons *by default* — omitting the field would silently keep
 * the default-on behavior. Undefined means omission is the correct off.
 */
export function explicitOffToken(provider: AIProvider, model: string): ReasoningEffort | undefined {
	switch (provider) {
		case 'googleai':
			// Gemini 2.5/3 think by default (dynamic budget / level). The backend
			// proxy maps 'none' to off on Flash, or the floor on Pro (only
			// reachable via a stale persisted preference — see canDisableReasoning).
			return 'none'
		case 'deepseek':
			return DEEPSEEK_OFF_SENTINEL
		case 'openai':
		case 'azure_openai':
		case 'azure_foundry':
			// gpt-5.1+ reasoning is off only via the explicit 'none' effort
			// (gpt-5.5 defaults to medium when the field is omitted).
			return /^gpt-5\./.test(baseModelId(model)) ? 'none' : undefined
		case 'openrouter':
			// OpenRouter validates effort against xhigh..minimal|none and
			// documents 'none' as disabling reasoning, translated per the
			// underlying provider — more reliable than omission, which keeps
			// reasoning-by-default models thinking.
			return 'none'
		default:
			return undefined
	}
}

/**
 * The effort to put on the wire for a given model selection. Same as
 * `resolveEffectiveReasoning`, plus: an explicit user "off" on a
 * reasoning-by-default provider resolves to that provider's disable token
 * instead of undefined — omitting the field would silently keep the provider's
 * default-on behavior, making the off switch a no-op.
 */
export function resolveRequestReasoning(
	modelProvider: ReasoningProviderModel
): ReasoningEffort | undefined {
	const effective = resolveEffectiveReasoning(modelProvider)
	if (effective !== undefined) {
		return effective
	}
	if (
		modelProvider.reasoning === REASONING_OFF &&
		supportsReasoning(modelProvider.provider, modelProvider.model)
	) {
		return explicitOffToken(modelProvider.provider, stripLegacyThinkingSuffix(modelProvider.model))
	}
	return undefined
}

export type ReasoningApiKind = 'anthropic' | 'responses' | 'completions' | 'deepseek' | 'mistral'

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
		case 'deepseek':
			// Same completions dialect, except "off" is a separate `thinking`
			// param — there is no effort token that disables thinking.
			if (effort === DEEPSEEK_OFF_SENTINEL) {
				return {
					...config,
					thinking: { type: 'disabled' }
				} as unknown as T
			}
			return {
				...config,
				reasoning_effort: effort
			} as unknown as T
		case 'mistral': {
			// Same completions dialect, but reasoning requests get stricter
			// sampling validation ("top_p must be 1 when using greedy sampling"
			// with our temperature 0) — strip sampling params like the
			// Anthropic adaptive-thinking path does.
			const { temperature: _t, top_p: _p, ...rest } = config as Record<string, any>
			return {
				...rest,
				reasoning_effort: effort
			} as unknown as T
		}
	}
}
