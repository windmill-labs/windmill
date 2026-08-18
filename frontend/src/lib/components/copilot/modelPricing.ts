import type { AIProvider, ModelPriceOverride } from '$lib/gen'
import { buildModelMatchers, matchModel, modelKey } from './modelConfig'

/** Rates in USD per million tokens, one per billed token class. */
export type ModelPrice = {
	input: number
	output: number
	cacheRead: number
	cacheWrite: number
}

export type ModelPriceSource = 'override' | 'builtin'

export type ResolvedModelPrice = {
	price: ModelPrice
	source: ModelPriceSource
}

/** What a chat spent on one model, in tokens. */
export type PricedTokens = {
	input: number
	cacheRead: number
	cacheWrite: number
	output: number
}

// Fallbacks for entries that do not price their cache separately: Anthropic reads a
// cached prefix at a tenth of the input rate and writes one at 1.25x (5-minute TTL,
// the default the chat uses). The read ratio is NOT universal — OpenAI and Google
// discount a cached read far less — so every non-Anthropic entry below states its own
// `cacheRead` rather than inheriting this. Providers whose caching is automatic never
// report a cache write, so their write rate is unused.
const CACHE_READ_RATIO = 0.1
const CACHE_WRITE_RATIO = 1.25

type PriceEntry = {
	input: number
	output: number
	cacheRead?: number
	cacheWrite?: number
}

/**
 * Published list prices, most specific entry first — the first name found in the
 * bare model id wins, so vendor-namespaced and date-suffixed ids
 * (anthropic/claude-opus-5, gpt-5-2026-01-01) still resolve. Matching is shared
 * with the context-window table via `buildModelMatchers`.
 *
 * This is a best-effort snapshot: vendors change rates, ship models faster than
 * this table is updated, and negotiated rates differ from list. A model that is
 * not listed resolves to undefined and is reported as unpriced rather than
 * guessed at, and any entry can be corrected per workspace from the AI settings.
 * Providers whose catalogue turns over too quickly to track (DeepSeek, Mistral,
 * Groq, TogetherAI, custom deployments) are deliberately absent.
 *
 * `null` marks a model that is known to exist but whose rates are not. Unpriced is
 * a supported state (the UI says so and points at the override); a confidently
 * wrong number is not — which is also why these matchers are built with
 * `strictVariants`, so an unlisted sub-model (`gpt-5-pro`) reports no rate instead
 * of inheriting its family's.
 *
 * One known gap the per-model shape cannot express: Anthropic's 1M-context beta
 * charges more above a threshold. Usage is aggregated per model before pricing, so
 * those requests are estimated at the standard tier and understate. An affected
 * workspace can set the higher rate as its override.
 */
const MODEL_PRICES: [name: string, price: PriceEntry | null][] = [
	// Anthropic — Opus 4.1 and older bill at the pre-4.5 Opus rate, so the family
	// fallback sits below the explicit entries rather than covering them.
	['claude-fable-5', { input: 10, output: 50 }],
	['claude-mythos-5', { input: 10, output: 50 }],
	['claude-opus-5', { input: 5, output: 25 }],
	['claude-opus-4-8', { input: 5, output: 25 }],
	['claude-opus-4-7', { input: 5, output: 25 }],
	['claude-opus-4-6', { input: 5, output: 25 }],
	['claude-opus-4-5', { input: 5, output: 25 }],
	['claude-opus-4-1', { input: 15, output: 75 }],
	['claude-opus-4', { input: 15, output: 75 }],
	// Sonnet 5 runs a promotional rate with a published end date, and
	// `claude-sonnet-latest` floats to it. Rates carry no date and apply at read
	// time, so either figure misstates one side of that boundary — unpriced until
	// the rate is a single number again.
	['claude-sonnet-5', null],
	['claude-sonnet-latest', null],
	['claude-sonnet-4-6', { input: 3, output: 15 }],
	['claude-sonnet-4-5', { input: 3, output: 15 }],
	['claude-sonnet-4', { input: 3, output: 15 }],
	['claude-haiku-4-5', { input: 1, output: 5 }],
	['claude-3-5-haiku', { input: 0.8, output: 4 }],
	['claude-opus', { input: 5, output: 25 }],
	['claude-sonnet', { input: 3, output: 15 }],
	['claude-haiku', { input: 1, output: 5 }],
	// OpenAI — the cached-input discount varies by family (a tenth on gpt-5, a
	// quarter on 4.1 and the o-series, half on 4o), so each entry carries its own
	// rate. There is no charge for writing the cache and no usage field reporting
	// one, so the write rate never applies. The -mini/-nano entries must precede
	// the family entry, which would otherwise claim them.
	// Revisions past gpt-5 are priced separately by OpenAI and are not tracked here.
	// The matcher's revision guard already keeps them off the family rate; these
	// entries stay so a revision the guard admits still resolves to no rate.
	['gpt-5.6', null],
	['gpt-5.5', null],
	['gpt-5.4', null],
	['gpt-5.2', null],
	['gpt-5.1', null],
	['gpt-5-mini', { input: 0.25, output: 2, cacheRead: 0.025 }],
	['gpt-5-nano', { input: 0.05, output: 0.4, cacheRead: 0.005 }],
	['gpt-5', { input: 1.25, output: 10, cacheRead: 0.125 }],
	['gpt-4.1-mini', { input: 0.4, output: 1.6, cacheRead: 0.1 }],
	['gpt-4.1-nano', { input: 0.1, output: 0.4, cacheRead: 0.025 }],
	['gpt-4.1', { input: 2, output: 8, cacheRead: 0.5 }],
	['gpt-4o-mini', { input: 0.15, output: 0.6, cacheRead: 0.075 }],
	['gpt-4o', { input: 2.5, output: 10, cacheRead: 1.25 }],
	['o4-mini', { input: 1.1, output: 4.4, cacheRead: 0.275 }],
	['o3-mini', { input: 1.1, output: 4.4, cacheRead: 0.55 }],
	['o3', { input: 2, output: 8, cacheRead: 0.5 }],
	// Google — Flash takes a flat rate and is priced; Pro is not, because both its
	// input and output roughly double above a 200k-token prompt and a per-model rate
	// cannot express a threshold. Explicit context caching also bills storage per hour,
	// which nothing here represents, so a workspace using it sees an underestimate.
	// Gemini 3.7 and 3.6 Flash run a promotional rate with an end date, and stay
	// unpriced for the same reason Sonnet 5 does.
	['gemini-2.5-flash-lite', { input: 0.1, output: 0.4, cacheRead: 0.01 }],
	['gemini-2.5-flash', { input: 0.3, output: 2.5, cacheRead: 0.03 }],
	['gemini-3.5-flash-lite', { input: 0.3, output: 2.5, cacheRead: 0.03 }],
	['gemini-3.5-flash', { input: 1.5, output: 9, cacheRead: 0.15 }],
	['gemini-3.7', null],
	['gemini-3.6', null],
	['gemini-3.1', null],
	['gemini-3', null],
	['gemini-2.5', null]
]

const MODEL_PRICE_MATCHERS = buildModelMatchers(
	MODEL_PRICES.map(([name, entry]): [string, ModelPrice | null] => [
		name,
		entry && {
			input: entry.input,
			output: entry.output,
			cacheRead: entry.cacheRead ?? entry.input * CACHE_READ_RATIO,
			cacheWrite: entry.cacheWrite ?? entry.input * CACHE_WRITE_RATIO
		}
	]),
	{ strictVariants: true }
)

export function getKnownModelPrice(model: string): ModelPrice | undefined {
	return matchModel(MODEL_PRICE_MATCHERS, model) ?? undefined
}

/**
 * Rates the API bounds on the way in — but an instance-level config is stored as an
 * untyped settings blob that bypasses that handler, so the reader enforces the same
 * bounds rather than rendering a negative, infinite or absurd total.
 */
const MAX_MODEL_RATE = 1000

function isUsableRate(rate: number | undefined): boolean {
	return rate === undefined || (Number.isFinite(rate) && rate >= 0 && rate <= MAX_MODEL_RATE)
}

/** What a cache rate falls back to when an override leaves it unset: the model's
 * own published multiple of the input rate where the table has one, and the input
 * rate itself where it does not, so an unstated discount is never borrowed from
 * another vendor. Shared with the rates editor, which shows these as placeholders. */
export function inheritedCacheRates(
	model: string,
	input: number
): { cacheRead: number; cacheWrite: number } {
	const builtin = getKnownModelPrice(model)
	return {
		cacheRead: input * (builtin ? builtin.cacheRead / builtin.input : 1),
		cacheWrite: input * (builtin ? builtin.cacheWrite / builtin.input : 1)
	}
}

/**
 * The rate a workspace should be billed at for one model: its override when an
 * admin set one, otherwise the published list price, otherwise nothing. An override
 * that omits a cache rate takes it from `inheritedCacheRates`.
 */
export function resolveModelPrice(
	provider: AIProvider | string,
	model: string,
	overrides: Record<string, ModelPriceOverride> | undefined
): ResolvedModelPrice | undefined {
	const builtin = getKnownModelPrice(model)
	const candidate = overrides?.[modelKey(provider, model)]
	const override =
		candidate &&
		isUsableRate(candidate.input) &&
		isUsableRate(candidate.output) &&
		isUsableRate(candidate.cache_read) &&
		isUsableRate(candidate.cache_write)
			? candidate
			: undefined
	if (override) {
		const inherited = inheritedCacheRates(model, override.input)
		return {
			source: 'override',
			price: {
				input: override.input,
				output: override.output,
				cacheRead: override.cache_read ?? inherited.cacheRead,
				cacheWrite: override.cache_write ?? inherited.cacheWrite
			}
		}
	}
	return builtin ? { source: 'builtin', price: builtin } : undefined
}

/** Cost in USD of `tokens` at `price`. */
export function estimateCost(tokens: PricedTokens, price: ModelPrice): number {
	return (
		(tokens.input * price.input +
			tokens.cacheRead * price.cacheRead +
			tokens.cacheWrite * price.cacheWrite +
			tokens.output * price.output) /
		1_000_000
	)
}

/** Tokens spent on one model, from a chat's running totals or the usage API. */
export type ModelSpend = {
	provider: string
	model: string
	tokens: PricedTokens
	/** What the provider billed, where it reports a figure. */
	reportedCostUsd?: number
}

export type Priced = {
	/** Undefined when no rate is known for the model — reported as unpriced, never guessed. */
	cost: number | undefined
	source: ModelPriceSource | 'reported' | undefined
}

export type PricedSpend<T extends ModelSpend> = {
	/** The input entries, each with its cost — callers carry their own fields through
	 * rather than zipping the result back against the input by index. */
	rows: (T & Priced)[]
	total: number
	/** True when at least one row has no rate, so `total` understates the truth. */
	hasUnpriced: boolean
	/** True when at least one row is a figure the provider billed rather than an estimate. */
	hasReported: boolean
}

/**
 * Cost a set of per-model token counts. A provider-reported figure always wins:
 * it is what was actually charged, where everything else is list price times
 * tokens. `source` says which, so a view never presents an estimate as a bill.
 */
export function priceSpend<T extends ModelSpend>(
	spend: T[],
	overrides: Record<string, ModelPriceOverride> | undefined
): PricedSpend<T> {
	let total = 0
	let hasUnpriced = false
	let hasReported = false
	const rows = spend.map((entry): T & Priced => {
		if (entry.reportedCostUsd !== undefined) {
			hasReported = true
			total += entry.reportedCostUsd
			return { ...entry, cost: entry.reportedCostUsd, source: 'reported' }
		}
		const resolved = resolveModelPrice(entry.provider, entry.model, overrides)
		if (!resolved) {
			hasUnpriced = true
			return { ...entry, cost: undefined, source: undefined }
		}
		const cost = estimateCost(entry.tokens, resolved.price)
		total += cost
		return { ...entry, cost, source: resolved.source }
	})
	return { rows, total, hasUnpriced, hasReported }
}

/**
 * Money, at the precision the amount deserves: sub-cent spend is where a chat
 * spends most of its life, and rounding it to `$0.00` would read as free.
 */
export function formatUsd(amount: number): string {
	if (amount === 0) return '$0'
	if (amount < 0.01) return `$${amount.toFixed(4)}`
	if (amount < 1) return `$${amount.toFixed(3)}`
	return `$${amount.toFixed(2)}`
}
