/** One page hit from a provider-side web search (OpenAI sources carry no title). */
export type WebSearchSource = {
	url: string
	title?: string
}

/** The result shape any tool returns to have it rendered as a web search card. */
export type WebSearchResult = {
	sources: WebSearchSource[]
	query?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** A link the card can render: the source list drops everything else, so a result whose
 * urls are relative or `javascript:` would show as an empty list in place of its own output. */
export function isRenderableSourceUrl(url: string): boolean {
	try {
		return ['http:', 'https:'].includes(new URL(url).protocol)
	} catch {
		return false
	}
}

/** An absent optional field reaches JSON as `null` from a Python tool and as nothing from a
 * TypeScript one, so both read as absent. */
function isOptionalString(value: unknown): boolean {
	return value === null || typeof value === 'string'
}

function isWebSearchSource(value: unknown): value is { url: string; title?: unknown } {
	return (
		isRecord(value) &&
		typeof value.url === 'string' &&
		isRenderableSourceUrl(value.url) &&
		Object.entries(value).every(
			([key, v]) => key === 'url' || (key === 'title' && isOptionalString(v))
		)
	)
}

/**
 * A tool result as a web search, or undefined when it is anything else. The keys must be
 * exactly `sources` and an optional `query`: the card renders only urls and titles, so a
 * result carrying more would lose it. Accepts the result as its JSON text too.
 */
export function webSearchResultOf(result: unknown): WebSearchResult | undefined {
	if (typeof result === 'string') {
		try {
			result = JSON.parse(result)
		} catch {
			return undefined
		}
	}
	if (!isRecord(result) || !Array.isArray(result.sources) || result.sources.length === 0) {
		return undefined
	}
	const sources = result.sources
	if (
		!sources.every(isWebSearchSource) ||
		!Object.entries(result).every(
			([key, v]) => key === 'sources' || (key === 'query' && isOptionalString(v))
		)
	) {
		return undefined
	}
	return {
		sources: sources.map((source) => ({
			url: source.url,
			title: typeof source.title === 'string' ? source.title : undefined
		})),
		query: typeof result.query === 'string' ? result.query : undefined
	}
}
