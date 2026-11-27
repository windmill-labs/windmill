import type { AIProviderModel } from '$lib/gen'
import type { ContextElement } from './context'
import type { Tool } from './shared'
import { groupingToolsOf, type GroupingTools } from './toolGroups'
import { assembleGlobalTools } from './global/globalAssembly'
import { flowTools } from './flow/core'
import { getAppTools } from './app/core'
import { prepareScriptTools } from './script/core'
import { navigatorTools } from './navigator/core'
import { askTools } from './ask/core'
import { fetchWebPageTool, searchWebTool } from './web/core'

/** Every tool any copilot mode can offer, each mode assembled with every option on. A
 * transcript outlives the mode and options its calls were made in (`change_mode` keeps the
 * transcript, an MCP server is removed, a pipeline editor closes), so grouping reads this
 * whole set rather than the tools assembled now. */
export function everyChatTool(): Tool<any>[] {
	return [
		...assembleGlobalTools({
			previewTools: true,
			pipelineFolders: [''],
			mcpServers: [{ path: '' }]
		}),
		...flowTools,
		...getAppTools(),
		...prepareScriptTools({ provider: 'openai', model: 'gpt-4o' } as AIProviderModel, 'bun', [
			{ type: 'db' } as ContextElement
		]),
		...navigatorTools,
		...askTools,
		// Added per request by withWebSearchFallback, so no assembly above carries them.
		searchWebTool,
		fetchWebPageTool
	]
}

let grouping: GroupingTools | undefined
export function chatGroupingTools(): GroupingTools {
	grouping ??= groupingToolsOf(everyChatTool())
	return grouping
}
