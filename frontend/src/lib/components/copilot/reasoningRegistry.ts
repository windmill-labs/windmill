import type { AIProvider, AIProviderModel } from '$lib/gen'
import { usesAnthropicMessagesApi } from './modelConfig'

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

/**
 * Azure AI Foundry hosts multiple model families under one provider, so reasoning
 * support follows the underlying model rather than the provider: Claude deployments
 * reason like the native Anthropic provider (adaptive thinking + `output_config.effort`),
 * everything else (gpt-5 / o-series) like OpenAI. Resolving to the owning family here
 * lets the rest of the registry keep its per-family logic unchanged.
 */
function reasoningProviderFamily(provider: AIProvider, model: string): AIProvider {
	if (provider === 'azure_foundry') {
		return usesAnthropicMessagesApi(provider, model) ? 'anthropic' : 'openai'
	}
	return provider
}

/**
 * Sentinel sent for the deepseek off case. It never reaches the wire as an
 * effort: the 'deepseek' branch of `applyReasoningToConfig` translates it to
 * the provider's `thinking: {type: "disabled"}` param (`reasoning_effort:
 * "none"` is rejected by their API).
 */
export const DEEPSEEK_OFF_SENTINEL: ReasoningEffort = 'none'

/**
 * Sentinel for the Anthropic off case. Like the DeepSeek one it never reaches
 * the wire as an effort: the 'anthropic' branch of `applyReasoningToConfig`
 * translates it to `thinking: {type: "disabled"}`, the only off that Opus and
 * Sonnet 5 respect.
 */
export const ANTHROPIC_OFF_SENTINEL: ReasoningEffort = 'none'

/**
 * What the registry knows about one group of models. The rows below are matched in
 * order against the lowercased model id, first match wins, so a narrower row goes above
 * the row it overrides. A model no row matches does not reason, as far as we know.
 */
type ReasoningRule = {
	match: RegExp
	/** The levels the UI offers. An unsupported level is a 400, so this is per model. */
	levels: readonly ReasoningEffort[]
	/**
	 * Whether "off" really stops the model reasoning. When false the UI offers no off:
	 * the provider would reject it, or coerce it to the lowest level.
	 */
	canDisable: boolean
	/**
	 * The effort to send for off on a model that reasons when the field is omitted.
	 * Unset where omission is already off. Always `'none'`: `requestsReasoning` reads
	 * that as off, and each wire format translates it (see `applyReasoningToConfig`).
	 */
	offToken?: ReasoningEffort
	/**
	 * gpt-5.5 and later refuse function tools on Chat Completions while they reason,
	 * even with the effort omitted (live-verified); the Responses API has no such limit.
	 */
	completionsToolsNeedOff?: boolean
}

const LOW_TO_HIGH = ['low', 'medium', 'high']
const LOW_TO_XHIGH = ['low', 'medium', 'high', 'xhigh']
const LOW_TO_MAX = ['low', 'medium', 'high', 'xhigh', 'max']
const MINIMAL_TO_HIGH = ['minimal', 'low', 'medium', 'high']

/**
 * Claude models whose thinking cannot be turned off: an explicit disable 400s. Fable,
 * Mythos, and the 5.x point releases (Sonnet 5.5, Opus 5.5), whose lowest setting is
 * adaptive thinking at `low` (live-verified). The version match stops at one digit so a
 * dated id (`claude-sonnet-5-20260101`) stays Sonnet 5.
 */
const CLAUDE_ALWAYS_THINKING = /fable|mythos|claude-(opus|sonnet)-5[-.][1-9](?!\d)/

// Anthropic ids are matched anywhere in the id: Bedrock prefixes them
// (`us.anthropic.claude-opus-4-6-v1`). Opus 4.5 and older reject adaptive thinking.
const ANTHROPIC_RULES: ReasoningRule[] = [
	{ match: CLAUDE_ALWAYS_THINKING, levels: LOW_TO_MAX, canDisable: false },
	// The 5 family thinks when the field is absent, so off is an explicit disable.
	{
		match: /claude-(opus|sonnet)-5/,
		levels: LOW_TO_MAX,
		canDisable: true,
		offToken: ANTHROPIC_OFF_SENTINEL
	},
	// 4.6-4.8 only think when asked, so omission is already off.
	{ match: /claude-opus-4-[78]/, levels: LOW_TO_MAX, canDisable: true },
	{ match: /claude-(opus|sonnet)-4-6/, levels: ['low', 'medium', 'high', 'max'], canDisable: true }
]

const BEDROCK_RULES: ReasoningRule[] = [
	ANTHROPIC_RULES[0],
	// AWS documents Sonnet 5 on Bedrock as always thinking, where the native API
	// accepts a disable for it.
	{ match: /claude-sonnet-5/, levels: LOW_TO_MAX, canDisable: false },
	...ANTHROPIC_RULES.slice(1)
]

// Anchored at the start or after a gateway's `vendor/`, and the major is one digit:
// Azure names gpt-3.5 `gpt-35-turbo`. `minimal` exists on gpt-5 only, `xhigh` from
// gpt-5.5, `max` from gpt-5.6.
const OPENAI_RULES: ReasoningRule[] = [
	// Live-verified: astra takes low..max only, where sol and luna also take `none`.
	{
		match: /(?:^|\/)gpt-6-astra/,
		levels: LOW_TO_MAX,
		canDisable: false,
		completionsToolsNeedOff: true
	},
	{
		match: /(?:^|\/)gpt-[6-9](?:[.:-]|$)/,
		levels: LOW_TO_MAX,
		canDisable: true,
		offToken: 'none',
		completionsToolsNeedOff: true
	},
	{
		match: /(?:^|\/)gpt-5\.6/,
		levels: LOW_TO_MAX,
		canDisable: true,
		offToken: 'none',
		completionsToolsNeedOff: true
	},
	{
		match: /(?:^|\/)gpt-5\.5/,
		levels: LOW_TO_XHIGH,
		canDisable: true,
		offToken: 'none',
		completionsToolsNeedOff: true
	},
	// gpt-5.1+ are off only through `none`: omitted, they reason at medium.
	{ match: /(?:^|\/)gpt-5\./, levels: LOW_TO_HIGH, canDisable: true, offToken: 'none' },
	// gpt-5 and the o-series reject `none` and reason when it is omitted.
	{ match: /(?:^|\/)gpt-5(?:[:-]|$)/, levels: MINIMAL_TO_HIGH, canDisable: false },
	{ match: /(?:^|\/)o\d/, levels: LOW_TO_HIGH, canDisable: false }
]

// Gemini 2.5/3 think by default; the backend proxy maps `none` to off on Flash, or to
// the floor on Pro, which enforces one (level `low` on 3.x, 128 tokens on 2.5).
// Gemini 3+ Flash / Flash-Lite accept `minimal`; 2.5 takes numeric budgets the proxy
// maps from three tiers.
const GEMINI_RULES: ReasoningRule[] = [
	{ match: /gemini-2\.5.*pro/, levels: LOW_TO_HIGH, canDisable: false, offToken: 'none' },
	{ match: /gemini-2\.5/, levels: LOW_TO_HIGH, canDisable: true, offToken: 'none' },
	{ match: /gemini-3.*pro/, levels: LOW_TO_HIGH, canDisable: false, offToken: 'none' },
	{ match: /gemini-3.*(flash|lite)/, levels: MINIMAL_TO_HIGH, canDisable: true, offToken: 'none' },
	{ match: /gemini-3/, levels: LOW_TO_HIGH, canDisable: true, offToken: 'none' }
]

const REASONING_RULES: Partial<Record<AIProvider, ReasoningRule[]>> = {
	anthropic: ANTHROPIC_RULES,
	aws_bedrock: BEDROCK_RULES,
	openai: OPENAI_RULES,
	azure_openai: OPENAI_RULES,
	googleai: GEMINI_RULES,
	deepseek: [
		// Every current API model takes reasoning_effort, but only two levels are real:
		// low/medium are server-mapped to high, xhigh to max. The retired
		// `deepseek-chat` alias means "non-thinking mode", so a saved selection on it
		// must not silently become a thinking request. Off is a separate `thinking` param.
		{
			match: /(?:^|\/)deepseek(?!-chat(?::|$))/,
			levels: ['high', 'max'],
			canDisable: true,
			offToken: DEEPSEEK_OFF_SENTINEL
		}
	],
	mistral: [
		// Only the ids verified to accept reasoning_effort (large, magistral, ministral
		// and pinned versions reject it), whose only effort besides off is `high`.
		{
			match: /(?:^|\/)mistral-(?:(?:small|medium)-latest(?::|$)|medium-3[-.]5)/,
			levels: ['high'],
			canDisable: true
		}
	],
	// OpenRouter validates effort against its own vocabulary (minimal..xhigh + none) and
	// translates it per underlying provider, which scopes the ladder: Anthropic gets all
	// five as budget ratios, OpenAI gets the token verbatim, DeepSeek server-maps. `none`
	// is its documented off, more reliable than omission; it can only disable a model
	// whose upstream can.
	openrouter: [
		{
			match: /claude-(opus|sonnet)-5[-.][1-9](?!\d)/,
			levels: ['minimal', ...LOW_TO_XHIGH],
			canDisable: false
		},
		{
			match: /claude-(opus|sonnet)-(4|5)/,
			levels: ['minimal', ...LOW_TO_XHIGH],
			canDisable: true,
			offToken: 'none'
		},
		...GEMINI_RULES,
		...OPENAI_RULES.map(({ completionsToolsNeedOff: _, ...rule }) => ({
			...rule,
			offToken: 'none'
		})),
		{ match: /deepseek-v4/, levels: LOW_TO_XHIGH.slice(2), canDisable: true, offToken: 'none' },
		// deepseek-r1, grok-4 and :thinking variants reason unconditionally.
		{
			match: /deepseek-r|grok-4|:thinking/,
			levels: LOW_TO_HIGH,
			canDisable: false,
			offToken: 'none'
		}
	]
}

/** The row that describes a model, and whether its provider family has rows at all. */
function findReasoningRule(
	provider: AIProvider,
	model: string
): { rule: ReasoningRule | undefined; known: boolean } {
	const rules = REASONING_RULES[reasoningProviderFamily(provider, model)]
	const id = stripLegacyThinkingSuffix(model).toLowerCase()
	return { rule: rules?.find((rule) => rule.match.test(id)), known: rules !== undefined }
}

/** Whether Chat Completions needs reasoning off for this model to take function tools. */
export function completionsRejectsToolsWithReasoning(provider: AIProvider, model: string): boolean {
	return findReasoningRule(provider, model).rule?.completionsToolsNeedOff ?? false
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
	/**
	 * Whether `supported` is an answer or an absence of one. The registry has rules per
	 * provider family and falls through to `false` for the rest — `customai` above all,
	 * which fronts any OpenAI-compatible endpoint and may well serve a thinking model. A
	 * caller that presents `supported: false` as a fact must check this first, or it tells
	 * the reader a model cannot think when all we know is that we have never heard of it.
	 */
	known: boolean
}

/** Resolve the reasoning capability of a model from the static registry. */
export function getReasoningCapability(provider: AIProvider, model: string): ReasoningCapability {
	const { rule, known } = findReasoningRule(provider, model)
	return rule
		? { supported: true, levels: [...rule.levels], canDisable: rule.canDisable, known }
		: { supported: false, levels: [], canDisable: false, known }
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
 * Disable token to forward when the user explicitly turns reasoning off on a
 * model that reasons *by default* — omitting the field would silently keep
 * the default-on behavior. Undefined means omission is the correct off, or that
 * the model cannot be turned off at all.
 */
export function explicitOffToken(provider: AIProvider, model: string): ReasoningEffort | undefined {
	return findReasoningRule(provider, model).rule?.offToken
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

/**
 * Whether a request carrying `effort`, as `resolveRequestReasoning` resolves it,
 * has the model reason. An effort sent to a model the registry does not mark as
 * reasoning-capable answers no: an explicit choice is always sent, but that does
 * not make the model think.
 */
export function requestsReasoning(
	provider: AIProvider,
	model: string,
	effort: ReasoningEffort | undefined
): boolean {
	return !!effort && effort !== 'none' && supportsReasoning(provider, model)
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
			// The disable must not carry an effort: Opus 5 rejects it at xhigh
			// and max, and dropping the field leaves the model at its default
			// effort, where the disable is accepted.
			if (effort === ANTHROPIC_OFF_SENTINEL) {
				return { ...config, thinking: { type: 'disabled' } } as unknown as T
			}
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
