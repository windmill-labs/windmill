import type { Tool } from '../shared'
import type { ChatCompletionTool } from 'openai/resources/index.mjs'
import { DocumentationService } from '$lib/gen'

// The docs corpus is self-hosted by the backend (a vendored snapshot embedded in
// the binary). Search/ranking and page rendering happen server-side, so these
// tools are thin wrappers over /api/docs/* — see backend `windmill-api/src/docs`.
// The same endpoints back the MCP `searchDocs`/`readDocsPage` tools and the
// `wmill docs` CLI, so all three return identical content.

const READ_DOCS_PAGE_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'read_docs_page',
		description:
			'Fetch the raw markdown of a single Windmill documentation page. Provide the `url` of a page found via search_docs (its Source URL). If the page is large, this returns its list of section headings instead of the full content; call again with the `section` argument set to one of those headings to read that section.',
		parameters: {
			type: 'object',
			properties: {
				url: {
					type: 'string',
					description:
						'The docs page to read, as a Source URL returned by search_docs (e.g. https://www.windmill.dev/docs/core_concepts/jobs). A bare path (e.g. /docs/core_concepts/jobs) is also accepted.'
				},
				section: {
					type: 'string',
					description:
						'Optional. A heading title from the page outline to read just that section instead of the full page.'
				}
			},
			required: ['url']
		}
	}
}

export const readDocsPageTool: Tool<{}> = {
	def: READ_DOCS_PAGE_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		const url = typeof args?.url === 'string' ? args.url : ''
		const section =
			typeof args?.section === 'string' && args.section.trim() ? args.section : undefined
		toolCallbacks.setToolStatus(toolId, {
			content: section ? `Reading docs section "${section}"...` : 'Reading documentation page...'
		})
		try {
			if (!url.trim()) {
				return 'No documentation page URL was provided. Provide a `url` — e.g. a `Source` URL returned by search_docs.'
			}
			const res = await DocumentationService.readDocsPage({ url, section })
			toolCallbacks.setToolStatus(toolId, { content: 'Read documentation page' })
			return res.text
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

const SEARCH_DOCS_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'search_docs',
		description:
			'Full-text search across the entire Windmill documentation. Provide one or more keywords; returns the most relevant docs pages, each with its Source URL and short matching snippets. Use this FIRST to find relevant pages by their content (a flag, function, error message, config key or concept). If the snippets answer the question, answer directly; otherwise call read_docs_page with a returned Source URL to read the full page or a section.',
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
			const res = await DocumentationService.searchDocs({ query })
			const count = res.results?.length ?? 0
			toolCallbacks.setToolStatus(toolId, {
				content: count > 0 ? `Found ${count} matching page(s)` : 'No matching pages found'
			})
			return res.text
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
