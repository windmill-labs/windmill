import type { Tool } from '../shared'
import type { ChatCompletionTool } from 'openai/resources/index.mjs'

const DOCS_ORIGIN = 'https://www.windmill.dev'
const LLMS_TXT_URL = `${DOCS_ORIGIN}/llms.txt`
const CACHE_TTL_MS = 15 * 60 * 1000
// Above this size, return an outline of the page's headings instead of the full
// content, prompting the model to request a specific section.
const FULL_PAGE_CHAR_LIMIT = 20_000

interface CacheEntry {
	expiresAt: number
	promise: Promise<string>
}

let llmsTxtCache: CacheEntry | undefined
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

const LIST_DOCS_PAGES_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'list_docs_pages',
		description:
			'Return the full Windmill documentation index (llms.txt): a list of every docs page with its title, URL and a short description. Call this FIRST, before read_docs_page, to discover which pages are relevant to the user request.',
		parameters: {
			type: 'object',
			properties: {},
			required: []
		}
	}
}

const READ_DOCS_PAGE_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'read_docs_page',
		description:
			'Fetch the raw markdown of a single Windmill documentation page. Provide the `path` (or full URL) of a page found via list_docs_pages. If the page is large, this returns its list of section headings instead of the full content; call again with the `section` argument set to one of those headings to read that section.',
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

export const listDocsPagesTool: Tool<{}> = {
	def: LIST_DOCS_PAGES_TOOL,
	fn: async ({ toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, { content: 'Listing documentation pages...' })
		try {
			const index = await fetchDocsIndex()
			toolCallbacks.setToolStatus(toolId, { content: 'Retrieved documentation index' })
			return index
		} catch (error) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Error listing documentation pages',
				error: 'Error listing documentation pages'
			})
			console.error('Error listing documentation pages:', error)
			const errorMessage =
				error instanceof Error ? error.message : 'An error occurred while listing documentation pages'
			return `Failed to list documentation pages: ${errorMessage}, pursuing with the user request...`
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
				return 'No documentation page path was provided. Provide a `path` from list_docs_pages.'
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
