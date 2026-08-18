/**
 * Resource types are named by abbreviation — `gdrive`, `gcal`, `s3` — so a product's real
 * name ("Google Drive", "Amazon S3") only ever appears in its description. Matching on the
 * name alone means searching "google" finds none of the Google integrations.
 */
export function resourceTypeSearchText(name: string, description?: string): string {
	return description ? `${name} ${description}` : name
}

function isWordStart(haystack: string, at: number): boolean {
	return at === 0 || !/[a-z0-9]/.test(haystack[at - 1])
}

/**
 * Sort key for one resource type against a search query, lowest first.
 *
 * Searching the description makes a query like "google" match a dozen types, most of which
 * only mention the product in passing — so the ranking has to put a match on the type's own
 * name above any description match, otherwise `googleai` lands below `anthropic`.
 * Ties break on where the match starts: a description opening with the query describes the
 * product, one mentioning it halfway through is an aside.
 *
 * Returns Number.MAX_SAFE_INTEGER when the query appears in neither field, so a caller
 * matching more loosely than a substring (uFuzzy) keeps those results last instead of
 * dropping them.
 */
export function resourceTypeMatchRank(
	name: string,
	description: string | undefined,
	query: string
): number {
	const q = query.trim().toLowerCase()
	if (q === '') return 0

	const n = name.toLowerCase()
	if (n === q) return 0

	const inName = n.indexOf(q)
	if (inName >= 0) {
		const tier = inName === 0 ? 1 : isWordStart(n, inName) ? 2 : 3
		return tier * 1e4 + Math.min(inName, 9999)
	}

	const d = (description ?? '').toLowerCase()
	const inDescription = d.indexOf(q)
	if (inDescription < 0) return Number.MAX_SAFE_INTEGER

	const tier = isWordStart(d, inDescription) ? 4 : 5
	return tier * 1e4 + Math.min(inDescription, 9999)
}

/**
 * Rank-sorts resource types by how well they match `query`, keeping the incoming order
 * for equal ranks (and for an empty query, where callers rely on their own ordering).
 */
export function sortResourceTypesByMatch<T>(
	items: T[],
	query: string,
	name: (item: T) => string,
	description: (item: T) => string | undefined
): T[] {
	if (query.trim() === '') return items
	return items
		.map((item, index) => ({
			item,
			index,
			rank: resourceTypeMatchRank(name(item), description(item), query)
		}))
		.sort((a, b) => a.rank - b.rank || a.index - b.index)
		.map((entry) => entry.item)
}

/** Human-facing label for a resource type: `git_repository` -> `Git repository`. */
export function resourceTypeLabel(name: string): string {
	const spaced = name.replace(/_/g, ' ')
	return spaced.charAt(0).toUpperCase() + spaced.slice(1)
}

/** "Add **a** Supabase resource" / "Add **an** Airtable resource". */
export function resourceTypeArticle(name: string): string {
	return /^[aeiou]/i.test(name) ? 'an' : 'a'
}

/** Drawer title for creating a resource, named after its type once one is picked. */
export function addResourceTitle(resourceType: string | undefined): string {
	return resourceType
		? `Add ${resourceTypeArticle(resourceType)} ${resourceTypeLabel(resourceType)} resource`
		: 'Add a resource'
}
