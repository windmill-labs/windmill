/**
 * The GLOBAL toolset's versions of tools the other chat modes share: the same definition,
 * run on the server like every other GLOBAL tool.
 */
import type { SessionTool } from '../sessionCapabilities'
import { createSearchHubScriptsTool, searchHubScriptsSchema } from '../shared'
import { createDbSchemaTool, npmTextLimit, searchNpmPackagesTool } from '../script/core'
import { readDocsPageTool, searchDocsTool } from '../docs/core'
import { runOnServer } from './backendSessionTools'

export const searchHubScriptsServerTool: SessionTool<{}> = {
	...createSearchHubScriptsTool(false),
	fn: async ({ args, workspace, toolId, toolCallbacks }) => {
		const { query, integration } = searchHubScriptsSchema.parse(args)
		const subject = query ? `"${query}"` : `the ${integration} integration`
		if (query || integration) {
			toolCallbacks.setToolStatus(toolId, { content: `Searching hub scripts for ${subject}...` })
		}
		const { text } = await runOnServer('search_hub_scripts', workspace, args)
		if (query || integration) {
			const found = JSON.parse(text).results.length
			toolCallbacks.setToolStatus(toolId, {
				content:
					found === 0
						? `No hub script found for ${subject}`
						: `Found ${found} hub script${found === 1 ? '' : 's'} for ${subject}`
			})
		}
		return text
	}
}

export const searchNpmPackagesServerTool: SessionTool<{}> = {
	...searchNpmPackagesTool,
	fn: async ({ args, workspace, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, { content: 'Searching for relevant packages...' })
		const { text } = await runOnServer('search_npm_packages', workspace, args, {
			npm_text_limit: npmTextLimit()
		})
		toolCallbacks.setToolStatus(toolId, { content: 'Retrieved relevant packages' })
		return text
	}
}

export const searchDocsServerTool: SessionTool<{}> = {
	...searchDocsTool,
	fn: async ({ args, workspace, toolId, toolCallbacks }) => {
		const query = typeof args?.query === 'string' ? args.query.trim() : ''
		toolCallbacks.setToolStatus(toolId, {
			content: query ? `Searching documentation for "${query}"...` : 'Searching documentation...'
		})
		const { text, effects } = await runOnServer('search_docs', workspace, args)
		if (text.startsWith('Failed to search documentation:')) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Error searching documentation',
				error: 'Error searching documentation'
			})
		} else if (query) {
			const count = Number(
				effects.flatMap((e) => (e.type === 'facts' ? [e.result_count] : []))[0] ?? 0
			)
			toolCallbacks.setToolStatus(toolId, {
				content: count > 0 ? `Found ${count} matching page(s)` : 'No matching pages found'
			})
		}
		return text
	}
}

export const readDocsPageServerTool: SessionTool<{}> = {
	...readDocsPageTool,
	fn: async ({ args, workspace, toolId, toolCallbacks }) => {
		const url = typeof args?.url === 'string' ? args.url : ''
		const section =
			typeof args?.section === 'string' && args.section.trim() ? args.section : undefined
		toolCallbacks.setToolStatus(toolId, {
			content: section ? `Reading docs section "${section}"...` : 'Reading documentation page...'
		})
		const { text } = await runOnServer('read_docs_page', workspace, args)
		if (text.startsWith('Failed to read documentation page:')) {
			toolCallbacks.setToolStatus(toolId, {
				content: 'Error reading documentation page',
				error: 'Error reading documentation page'
			})
		} else if (url.trim()) {
			toolCallbacks.setToolStatus(toolId, { content: 'Read documentation page' })
		}
		return text
	}
}

export function dbSchemaServerTool(description: string): SessionTool<{}> {
	return {
		...createDbSchemaTool<{}>({ description, updateEditorCache: false }),
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			if (!args.resourcePath) {
				throw new Error('Database path not provided')
			}
			toolCallbacks.setToolStatus(toolId, {
				content: 'Getting database schema for ' + args.resourcePath + '...'
			})
			const { text } = await runOnServer('get_db_schema', workspace, args)
			toolCallbacks.setToolStatus(toolId, {
				content: 'Retrieved database schema for ' + args.resourcePath
			})
			return text
		}
	}
}
