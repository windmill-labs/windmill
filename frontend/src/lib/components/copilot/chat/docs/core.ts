import type { Tool } from '../shared'
import type { ChatCompletionTool } from 'openai/resources/index.mjs'

const DOCS_ORIGIN = 'https://www.windmill.dev'
const LLMS_TXT_URL = `${DOCS_ORIGIN}/llms.txt`
const LLMS_FULL_TXT_URL = `${DOCS_ORIGIN}/llms-full.txt`
const CACHE_TTL_MS = 15 * 60 * 1000
// Above this size, return an outline of the page's headings instead of the full
// content, prompting the model to request a specific section.
const FULL_PAGE_CHAR_LIMIT = 20_000

// search_docs result caps — keep the returned payload small (the whole point of
// search vs. dumping the index or full pages is token economy).
const SEARCH_MAX_PAGES = 8
const SEARCH_MAX_SNIPPETS_PER_PAGE = 3
const SEARCH_MAX_SNIPPET_CHARS = 200

interface CacheEntry {
	expiresAt: number
	promise: Promise<string>
}

let llmsTxtCache: CacheEntry | undefined
let llmsFullTxtCache: CacheEntry | undefined
const pageCache = new Map<string, CacheEntry>()

/**
 * Fetches the docs index (llms.txt) listing every documentation page. Cached at
 * module level with a TTL so repeated tool calls within a session reuse it.
 */
export async function fetchDocsIndex(): Promise<string> {
	const now = Date.now()
	if (llmsTxtCache && llmsTxtCache.expiresAt > now) {
		return llmsTxtCache.promise
	}

	const promise = fetchText(LLMS_TXT_URL).catch((error) => {
		// Drop the failed promise from the cache so the next call retries.
		if (llmsTxtCache?.promise === promise) {
			llmsTxtCache = undefined
		}
		throw error
	})
	llmsTxtCache = { expiresAt: now + CACHE_TTL_MS, promise }
	return promise
}

/**
 * Fetches the full documentation corpus (llms-full.txt): every page concatenated
 * into one document, each delimited by a `Source: <url>` line. ~2 MB. Cached at
 * module level with a TTL. Mirrors fetchDocsIndex; used by search_docs to grep
 * the whole corpus in a single fetch.
 */
export async function fetchDocsFullText(): Promise<string> {
	const now = Date.now()
	if (llmsFullTxtCache && llmsFullTxtCache.expiresAt > now) {
		return llmsFullTxtCache.promise
	}

	const promise = fetchText(LLMS_FULL_TXT_URL).catch((error) => {
		if (llmsFullTxtCache?.promise === promise) {
			llmsFullTxtCache = undefined
		}
		throw error
	})
	llmsFullTxtCache = { expiresAt: now + CACHE_TTL_MS, promise }
	return promise
}

/**
 * Fetches a single docs page as raw markdown. `path` may be a full URL or a
 * /docs/... path; it is normalized to a `.md` URL. Cached per resolved URL.
 */
export async function fetchDocsPage(path: string): Promise<string> {
	const url = normalizeDocsUrl(path)
	const now = Date.now()
	const cached = pageCache.get(url)
	if (cached && cached.expiresAt > now) {
		return cached.promise
	}

	const promise = fetchText(url)
		.then((content) => sanitizeDocsMarkdownLinks(content, url))
		.catch((error) => {
			if (pageCache.get(url)?.promise === promise) {
				pageCache.delete(url)
			}
			throw error
		})
	pageCache.set(url, { expiresAt: now + CACHE_TTL_MS, promise })
	return promise
}

async function fetchText(url: string): Promise<string> {
	const response = await fetch(url)
	if (!response.ok) {
		throw new Error(`Request to ${url} failed with status ${response.status}`)
	}
	return await response.text()
}

/**
 * Normalizes a user/model-supplied docs reference to a fully-qualified `.md`
 * URL on the docs origin. Accepts:
 *   - `https://www.windmill.dev/docs/core_concepts/jobs`
 *   - `/docs/core_concepts/jobs.md`
 *   - `docs/core_concepts/jobs`
 */
export function normalizeDocsUrl(input: string): string {
	let value = input.trim()

	if (/^https?:\/\//i.test(value)) {
		// Strip the origin so we can re-anchor to DOCS_ORIGIN and normalize the path.
		try {
			const parsed = new URL(value)
			value = parsed.pathname
		} catch {
			// Fall through and treat as a path.
		}
	}

	// Drop any query string or hash fragment.
	value = value.split('#')[0].split('?')[0]

	if (!value.startsWith('/')) {
		value = `/${value}`
	}

	// Strip a trailing slash (but keep the leading one).
	if (value.length > 1 && value.endsWith('/')) {
		value = value.slice(0, -1)
	}

	// Relative links inside the raw markdown reference docusaurus source files
	// (e.g. `13_flow_branches.mdx`), but the published routes drop the numeric
	// ordering prefixes and use `.md`.
	value = stripDocsPathPrefixes(value)
	if (value.endsWith('.mdx')) {
		value = value.slice(0, -1)
	}

	if (!value.endsWith('.md')) {
		value = `${value}.md`
	}

	return `${DOCS_ORIGIN}${value}`
}

/**
 * The canonical published URL a model should cite for a docs page (the `.md`
 * fetch URL without the suffix), e.g. `https://www.windmill.dev/docs/flows/retries`.
 */
export function canonicalDocsPageUrl(path: string): string {
	return normalizeDocsUrl(path).replace(/\.md$/i, '')
}

/**
 * Strips docusaurus numeric ordering prefixes (`13_`, `8-`) from each segment of
 * a docs path so it matches the published route. Operates on the path only.
 */
function stripDocsPathPrefixes(path: string): string {
	return path
		.split('/')
		.map((segment) => segment.replace(/^\d+[_-]/, ''))
		.join('/')
}

/**
 * Rewrites relative/source-file doc links inside raw page markdown to canonical
 * published URLs, so the model never echoes a docusaurus source path (e.g.
 * `./13_flow_branches.mdx`) into its answer as a broken link. Resolves each link
 * relative to the page it came from, strips numeric ordering prefixes, and drops
 * the `.md`/`.mdx` extension. Non-doc links (external, images, anchors) are left
 * untouched.
 */
export function sanitizeDocsMarkdownLinks(content: string, pageUrl: string): string {
	return content.replace(/\]\(([^)\s]+?)(\s+"[^"]*")?\)/g, (match, target: string, title) => {
		if (!/\.mdx?($|[#?])/i.test(target)) {
			// Only rewrite links to docusaurus source files (.md/.mdx); leave
			// images, external URLs and bare anchors untouched.
			return match
		}
		if (/(^|\/)\.\.\//.test(target)) {
			// `../` cross-directory links are authored against the docusaurus
			// source tree, whose depth differs from the published URL, so strict
			// resolution is unreliable. Leave them for the canonical-URL header to
			// disambiguate rather than risk rewriting to a wrong path.
			return match
		}
		let resolved: URL
		try {
			resolved = new URL(target, pageUrl)
		} catch {
			return match
		}
		if (resolved.origin !== DOCS_ORIGIN || !resolved.pathname.startsWith('/docs/')) {
			return match
		}
		const pathname = stripDocsPathPrefixes(resolved.pathname).replace(/\.mdx?$/i, '')
		return `](${DOCS_ORIGIN}${pathname}${resolved.hash}${title ?? ''})`
	})
}

export interface DocsHeading {
	level: number
	title: string
	/** Character offset of the start of the heading line within the document. */
	startIndex: number
}

/**
 * Parses the markdown headings (`#`–`####`) of a docs page, ignoring any
 * heading-like lines that appear inside fenced code blocks (``` fences), which
 * are common in docs pages (e.g. `# comment` inside a python sample).
 */
export function parseDocsHeadings(content: string): DocsHeading[] {
	const headings: DocsHeading[] = []
	let offset = 0
	let inFence = false
	let fenceMarker = ''

	const lines = content.split('\n')
	for (const line of lines) {
		const fence = matchFence(line)
		if (fence) {
			if (!inFence) {
				inFence = true
				fenceMarker = fence
			} else if (line.trimStart().startsWith(fenceMarker)) {
				inFence = false
				fenceMarker = ''
			}
			offset += line.length + 1
			continue
		}

		if (!inFence) {
			const match = /^(#{1,4})\s+(.*\S)\s*$/.exec(line)
			if (match) {
				headings.push({
					level: match[1].length,
					title: match[2].trim(),
					startIndex: offset
				})
			}
		}

		offset += line.length + 1
	}

	return headings
}

function matchFence(line: string): string | undefined {
	const trimmed = line.trimStart()
	const match = /^(`{3,}|~{3,})/.exec(trimmed)
	return match ? match[1] : undefined
}

/**
 * Builds a human-readable outline of a page's headings, including an approximate
 * character size for each section. Used when a page is too large to return whole.
 */
export function buildDocsOutline(content: string): string {
	const headings = parseDocsHeadings(content)
	if (headings.length === 0) {
		return '(no markdown headings found on this page)'
	}

	const lines = headings.map((heading, index) => {
		const sectionEnd = sectionEndIndex(content, headings, index)
		const approxChars = sectionEnd - heading.startIndex
		const indent = '  '.repeat(Math.max(0, heading.level - 1))
		return `${indent}- ${heading.title} (~${approxChars} chars)`
	})

	return lines.join('\n')
}

function sectionEndIndex(content: string, headings: DocsHeading[], index: number): number {
	const heading = headings[index]
	// A section ends at the next heading of the same or higher (shallower) level.
	for (let i = index + 1; i < headings.length; i++) {
		if (headings[i].level <= heading.level) {
			return headings[i].startIndex
		}
	}
	return content.length
}

/** Normalizes a heading title for tolerant, case/punctuation-insensitive matching. */
function normalizeHeadingTitle(title: string): string {
	return title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, ' ')
		.trim()
}

/**
 * Extracts the content of the section whose heading matches `section`, from the
 * matching heading up to the next heading of the same or higher level. Matching
 * is case-insensitive and tolerant of minor punctuation differences. Returns
 * `undefined` when no heading matches.
 */
export function extractDocsSection(content: string, section: string): string | undefined {
	const headings = parseDocsHeadings(content)
	const target = normalizeHeadingTitle(section)
	if (target.length === 0) {
		return undefined
	}

	let matchIndex = headings.findIndex(
		(heading) => normalizeHeadingTitle(heading.title) === target
	)
	if (matchIndex === -1) {
		// Fall back to a contains match so "Result streaming" matches "Result".
		matchIndex = headings.findIndex((heading) =>
			normalizeHeadingTitle(heading.title).includes(target)
		)
	}
	if (matchIndex === -1) {
		return undefined
	}

	const start = headings[matchIndex].startIndex
	const end = sectionEndIndex(content, headings, matchIndex)
	return content.slice(start, end).trim()
}

const READ_DOCS_PAGE_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'read_docs_page',
		description:
			'Fetch the raw markdown of a single Windmill documentation page. Provide the `path` (or full URL) of a page found via search_docs. If the page is large, this returns its list of section headings instead of the full content; call again with the `section` argument set to one of those headings to read that section.',
		parameters: {
			type: 'object',
			properties: {
				path: {
					type: 'string',
					description:
						'The docs page to read, as a path (e.g. /docs/core_concepts/jobs) or full URL (e.g. https://www.windmill.dev/docs/core_concepts/jobs).'
				},
				section: {
					type: 'string',
					description:
						'Optional. A heading title from the page outline to read just that section instead of the full page.'
				}
			},
			required: ['path']
		}
	}
}

export const readDocsPageTool: Tool<{}> = {
	def: READ_DOCS_PAGE_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		const path = typeof args?.path === 'string' ? args.path : ''
		const section = typeof args?.section === 'string' && args.section.trim() ? args.section : undefined
		toolCallbacks.setToolStatus(toolId, {
			content: section ? `Reading docs section "${section}"...` : 'Reading documentation page...'
		})
		try {
			if (!path.trim()) {
				return 'No documentation page path was provided. Provide a `path` — e.g. a `Source` URL returned by search_docs.'
			}
			const content = await fetchDocsPage(path)
			toolCallbacks.setToolStatus(toolId, { content: 'Read documentation page' })
			const canonicalUrl = canonicalDocsPageUrl(path)
			const header = `Source page — cite this URL when referencing this page: ${canonicalUrl}\n\n`
			return header + renderDocsPageResult(content, section)
		} catch (error) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Error reading documentation page',
				error: 'Error reading documentation page'
			})
			console.error('Error reading documentation page:', error)
			const errorMessage =
				error instanceof Error ? error.message : 'An error occurred while reading the documentation page'
			return `Failed to read documentation page: ${errorMessage}, pursuing with the user request...`
		}
	}
}

/**
 * Decides what to return for read_docs_page: a requested section, the full page,
 * or an outline asking the model to pick a section.
 */
export function renderDocsPageResult(content: string, section?: string): string {
	if (section) {
		const extracted = extractDocsSection(content, section)
		if (extracted !== undefined) {
			return extracted
		}
		return [
			`No section matching "${section}" was found on this page. Available sections:`,
			'',
			buildDocsOutline(content)
		].join('\n')
	}

	// Gate on the page body only; the caller may prepend a short "Source page"
	// header, so the returned payload can exceed this limit by that header's
	// length. This threshold only decides whole-page vs. outline, so the small
	// overshoot is immaterial.
	if (content.length <= FULL_PAGE_CHAR_LIMIT) {
		return content
	}

	return [
		'This documentation page is large. Below is its list of sections with approximate sizes.',
		'Call read_docs_page again with the same path and a `section` set to one of these headings to read that section.',
		'',
		buildDocsOutline(content)
	].join('\n')
}

// ---------------------------------------------------------------------------
// Full-text docs search (search_docs)
//
// Discovery primitive for the `search` ask variant: instead of dumping the whole
// llms.txt index, grep the full corpus (llms-full.txt) for the user's keywords
// and return only small matching snippets plus each page's `Source:` URL. The
// model then cites that URL directly or passes it to read_docs_page for more.
// ---------------------------------------------------------------------------

const SOURCE_LINE_RE = /^Source:\s*(\S+)\s*$/
// In llms-full.txt every page's `Source:` line is preceded by a category-header
// lead-in: `...page body...\n\n---\n\n## <Category>\n\nSource: <url>`. Splitting
// on `Source:` lines leaves that lead-in on the *previous* page, so strip a
// trailing `---` + level-2-heading block to avoid mis-attributing the next
// page's category title to the previous page.
const TRAILING_LEAD_IN_RE = /\n+-{3,}[ \t]*\n+#{2}[ \t]+.*[ \t]*\n*$/

export interface DocsFullPage {
	url: string
	title: string
	body: string
}

export interface DocsSearchResult {
	url: string
	title: string
	/** Higher = more relevant. Distinct query terms matched dominate raw occurrences. */
	score: number
	snippets: string[]
}

/**
 * Splits the llms-full.txt corpus into per-page records keyed by the `Source:`
 * URL. Content before the first `Source:` line (the corpus preamble) is dropped.
 */
export function parseDocsFullText(fullText: string): DocsFullPage[] {
	const pages: DocsFullPage[] = []
	let url: string | undefined
	let buffer: string[] = []

	const flush = () => {
		if (url === undefined) {
			return
		}
		const body = buffer.join('\n').replace(TRAILING_LEAD_IN_RE, '').trim()
		if (body.length > 0) {
			pages.push({ url, title: firstHeading(body) ?? url, body })
		}
	}

	for (const line of fullText.split('\n')) {
		const match = SOURCE_LINE_RE.exec(line)
		if (match) {
			flush()
			url = match[1]
			buffer = []
			continue
		}
		if (url !== undefined) {
			buffer.push(line)
		}
	}
	flush()
	return pages
}

function firstHeading(body: string): string | undefined {
	for (const line of body.split('\n')) {
		const match = /^#{1,6}\s+(.*\S)\s*$/.exec(line)
		if (match) {
			return match[1].trim()
		}
	}
	return undefined
}

/**
 * Ranks docs pages for a keyword query. The query is split into distinct terms;
 * a page's score is `distinctTermsMatched` (dominant) then total occurrences.
 * Pages covering every term are preferred over partial matches. Each result
 * carries up to `maxSnippetsPerPage` of its most term-dense lines.
 */
export function searchDocsPages(
	pages: DocsFullPage[],
	query: string,
	opts: { maxPages?: number; maxSnippetsPerPage?: number; maxSnippetChars?: number } = {}
): DocsSearchResult[] {
	const maxPages = opts.maxPages ?? SEARCH_MAX_PAGES
	const maxSnippetsPerPage = opts.maxSnippetsPerPage ?? SEARCH_MAX_SNIPPETS_PER_PAGE
	const maxSnippetChars = opts.maxSnippetChars ?? SEARCH_MAX_SNIPPET_CHARS

	const terms = tokenizeQuery(query)
	if (terms.length === 0) {
		return []
	}

	interface Scored extends DocsSearchResult {
		distinctTerms: number
		order: number
	}
	const scored: Scored[] = []

	pages.forEach((page, order) => {
		const lowerBody = page.body.toLowerCase()
		let distinctTerms = 0
		let occurrences = 0
		for (const term of terms) {
			const count = countOccurrences(lowerBody, term)
			if (count > 0) {
				distinctTerms += 1
				occurrences += count
			}
		}
		if (distinctTerms === 0) {
			return
		}
		scored.push({
			url: page.url,
			title: page.title,
			// distinctTerms dominates so a page matching all terms always outranks
			// one matching fewer, regardless of raw occurrence counts.
			score: distinctTerms * 1_000_000 + occurrences,
			distinctTerms,
			order,
			snippets: selectSnippets(page.body, terms, maxSnippetsPerPage, maxSnippetChars)
		})
	})

	// Prefer pages that cover every query term; fall back to partial matches only
	// when nothing covers all of them.
	const fullCoverage = scored.filter((entry) => entry.distinctTerms === terms.length)
	const pool = fullCoverage.length > 0 ? fullCoverage : scored

	pool.sort((a, b) => b.score - a.score || a.order - b.order)

	return pool
		.slice(0, maxPages)
		.map(({ url, title, score, snippets }) => ({ url, title, score, snippets }))
}

/** Splits a query into distinct, lowercased, non-empty terms. */
function tokenizeQuery(query: string): string[] {
	return Array.from(
		new Set(
			query
				.toLowerCase()
				.split(/\s+/)
				.map((term) => term.trim())
				.filter((term) => term.length > 0)
		)
	)
}

function countOccurrences(haystack: string, needle: string): number {
	if (needle.length === 0) {
		return 0
	}
	let count = 0
	let index = haystack.indexOf(needle)
	while (index !== -1) {
		count += 1
		index = haystack.indexOf(needle, index + needle.length)
	}
	return count
}

/**
 * Picks the most term-dense lines of a page body as snippets, in document order,
 * deduped, each trimmed to `maxChars` around the first matched term.
 */
function selectSnippets(
	body: string,
	terms: string[],
	maxSnippets: number,
	maxChars: number
): string[] {
	interface LineHit {
		text: string
		distinct: number
		order: number
	}
	const hits: LineHit[] = []

	body.split('\n').forEach((line, order) => {
		const lower = line.toLowerCase()
		let distinct = 0
		for (const term of terms) {
			if (lower.includes(term)) {
				distinct += 1
			}
		}
		if (distinct === 0) {
			return
		}
		const text = makeSnippet(line, terms, maxChars)
		if (text.length > 0) {
			hits.push({ text, distinct, order })
		}
	})

	hits.sort((a, b) => b.distinct - a.distinct || a.order - b.order)

	const seen = new Set<string>()
	const result: string[] = []
	for (const hit of hits) {
		if (seen.has(hit.text)) {
			continue
		}
		seen.add(hit.text)
		result.push(hit.text)
		if (result.length >= maxSnippets) {
			break
		}
	}
	return result
}

/**
 * Collapses a matched line to a single-line snippet of at most `maxChars`,
 * windowed around the first matched term (with ellipses) when the line is long.
 */
export function makeSnippet(line: string, terms: string[], maxChars: number): string {
	const collapsed = line.replace(/\s+/g, ' ').trim()
	if (collapsed.length <= maxChars) {
		return collapsed
	}

	const lower = collapsed.toLowerCase()
	let firstIndex = -1
	for (const term of terms) {
		const index = lower.indexOf(term)
		if (index !== -1 && (firstIndex === -1 || index < firstIndex)) {
			firstIndex = index
		}
	}
	if (firstIndex === -1) {
		return `${collapsed.slice(0, maxChars).trimEnd()}…`
	}

	const start = Math.max(0, firstIndex - Math.floor(maxChars / 3))
	const end = Math.min(collapsed.length, start + maxChars)
	const prefix = start > 0 ? '…' : ''
	const suffix = end < collapsed.length ? '…' : ''
	return `${prefix}${collapsed.slice(start, end).trim()}${suffix}`
}

export interface DocsIndexEntry {
	title: string
	url: string
	description: string
}

// A line in llms.txt: `- [Title](https://.../page.md): question-phrased description`.
const INDEX_ENTRY_RE = /^\s*-\s*\[([^\]]+)\]\(([^)\s]+)\)\s*:?\s*(.*)$/

/** Parses the llms.txt index into per-page entries (title, URL, description). */
export function parseDocsIndex(indexText: string): DocsIndexEntry[] {
	const entries: DocsIndexEntry[] = []
	for (const line of indexText.split('\n')) {
		const match = INDEX_ENTRY_RE.exec(line)
		if (!match) {
			continue
		}
		const [, title, url, description] = match
		if (!url.includes('/docs/')) {
			continue
		}
		entries.push({ title: title.trim(), url: url.trim(), description: description.trim() })
	}
	return entries
}

/**
 * Ranks index entries for a query by matching its terms against each entry's
 * title and description. Title matches weigh more than description matches.
 * The description becomes the result's single snippet. This recovers the
 * "named feature" discovery that full-text grep misses when the model searches
 * the wrong keywords (e.g. finding "AI agents" for "LLM decides which script").
 */
export function searchDocsIndex(
	entries: DocsIndexEntry[],
	query: string,
	opts: { maxPages?: number } = {}
): DocsSearchResult[] {
	const maxPages = opts.maxPages ?? SEARCH_MAX_PAGES
	const terms = tokenizeQuery(query)
	if (terms.length === 0) {
		return []
	}

	interface Scored extends DocsSearchResult {
		distinctTerms: number
		order: number
	}
	const scored: Scored[] = []

	entries.forEach((entry, order) => {
		const title = entry.title.toLowerCase()
		const description = entry.description.toLowerCase()
		let distinctTerms = 0
		let score = 0
		for (const term of terms) {
			const inTitle = title.includes(term)
			const inDescription = description.includes(term)
			if (inTitle || inDescription) {
				distinctTerms += 1
				score += (inTitle ? 5 : 0) + (inDescription ? 1 : 0)
			}
		}
		if (distinctTerms === 0) {
			return
		}
		scored.push({
			url: entry.url,
			title: entry.title,
			score: distinctTerms * 1_000_000 + score,
			distinctTerms,
			order,
			snippets: entry.description ? [entry.description] : []
		})
	})

	const fullCoverage = scored.filter((entry) => entry.distinctTerms === terms.length)
	const pool = fullCoverage.length > 0 ? fullCoverage : scored
	pool.sort((a, b) => b.score - a.score || a.order - b.order)

	return pool
		.slice(0, maxPages)
		.map(({ url, title, score, snippets }) => ({ url, title, score, snippets }))
}

/** Strips the `.md` suffix and trailing slash so index/body URLs dedupe. */
function canonicalSearchUrl(url: string): string {
	return url.replace(/\.md$/i, '').replace(/\/$/, '')
}

/**
 * Merges full-text (body) results with index-description results. Body matches
 * come first (concrete content hits), then index-only matches fill remaining
 * slots — so a named feature surfaced only by its index entry still appears even
 * when body grep landed on the wrong pages.
 */
export function mergeDocsSearchResults(
	bodyResults: DocsSearchResult[],
	indexResults: DocsSearchResult[],
	maxPages = SEARCH_MAX_PAGES
): DocsSearchResult[] {
	const seen = new Set(bodyResults.map((result) => canonicalSearchUrl(result.url)))
	const merged = [...bodyResults]
	for (const entry of indexResults) {
		const key = canonicalSearchUrl(entry.url)
		if (seen.has(key)) {
			continue
		}
		seen.add(key)
		merged.push(entry)
	}
	return merged.slice(0, maxPages)
}

/** Renders search results as the string returned to the model. */
export function formatDocsSearchResults(query: string, results: DocsSearchResult[]): string {
	if (results.length === 0) {
		return `No documentation pages matched "${query}". Try fewer or more general keywords (a single distinctive term often works best).`
	}

	const blocks = results.map((result) => {
		const lines = [`## ${result.title}`, `Source: ${result.url}`]
		for (const snippet of result.snippets) {
			lines.push(`  - ${snippet}`)
		}
		return lines.join('\n')
	})

	return [
		`Found ${results.length} documentation page(s) matching "${query}", most relevant first:`,
		'',
		blocks.join('\n\n'),
		'',
		'Cite the exact "Source" URL when referencing a page. If these snippets are not enough, call read_docs_page with a Source URL to read the full page or a section.'
	].join('\n')
}

const SEARCH_DOCS_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'search_docs',
		description:
			'Full-text search across the entire Windmill documentation. Provide one or more keywords; returns the most relevant docs pages, each with its Source URL and short matching snippets. Use this FIRST to find relevant pages by their content (a flag, function, error message, config key or concept). If the snippets answer the question, answer directly; otherwise call read_docs_page with a returned Source URL to read more.',
		parameters: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description:
						'Keywords to search for in the documentation body, e.g. "chromium worker tag" or "retry exponential backoff". Fewer, more distinctive words match better.'
				}
			},
			required: ['query']
		}
	}
}

export const searchDocsTool: Tool<{}> = {
	def: SEARCH_DOCS_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		const query = typeof args?.query === 'string' ? args.query.trim() : ''
		toolCallbacks.setToolStatus(toolId, {
			content: query ? `Searching documentation for "${query}"...` : 'Searching documentation...'
		})
		try {
			if (!query) {
				return 'No search query was provided. Provide a `query` of one or more keywords.'
			}
			const bodyResults = searchDocsPages(parseDocsFullText(await fetchDocsFullText()), query, {
				maxPages: 5
			})
			// Also match the (small) index titles/descriptions to surface named
			// features that body grep misses. Best-effort: a failed index fetch
			// still leaves full-text results.
			let indexResults: DocsSearchResult[] = []
			try {
				indexResults = searchDocsIndex(parseDocsIndex(await fetchDocsIndex()), query, {
					maxPages: 4
				})
			} catch (indexError) {
				console.error('Error searching documentation index:', indexError)
			}
			const results = mergeDocsSearchResults(bodyResults, indexResults)
			toolCallbacks.setToolStatus(toolId, {
				content:
					results.length > 0 ? `Found ${results.length} matching page(s)` : 'No matching pages found'
			})
			return formatDocsSearchResults(query, results)
		} catch (error) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Error searching documentation',
				error: 'Error searching documentation'
			})
			console.error('Error searching documentation:', error)
			const errorMessage =
				error instanceof Error ? error.message : 'An error occurred while searching the documentation'
			return `Failed to search documentation: ${errorMessage}, pursuing with the user request...`
		}
	}
}
