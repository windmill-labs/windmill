import { describe, expect, it, vi } from 'vitest'

// `agentToolUtils` reaches the copilot bundle, and Monaco's CSS with it, through this one import.
// Only `createAiAgentTool` reads it, and nothing below does.
vi.mock('../aiProviderStorage', () => ({ loadStoredConfig: () => undefined }))

import { getToolNameError, toolEnabledName, WEBSEARCH_ENABLED_NAME } from './agentToolUtils'

/**
 * The names this returns are the ones `enabled_tools` holds and `tool_enabled_name` in
 * `ai_executor.rs` matches against, so the two have to agree: a name only one side produces
 * silently drops the tool from every run that narrows.
 */
describe('toolEnabledName', () => {
	it('names a flow module tool by the name the model is shown', () => {
		expect(
			toolEnabledName({ id: 'a', summary: 'get_user', value: { tool_type: 'flowmodule' } } as any)
		).toBe('get_user')
	})

	it('names an MCP server by its bare path, never the summary two servers may share', () => {
		// `$res:` and all is how the roster stores it, but a name carrying that prefix is resolved to
		// the resource itself before the worker sees it, so the list can only hold the bare path.
		expect(
			toolEnabledName({
				id: 'm',
				summary: 'github',
				value: { tool_type: 'mcp', resource_path: '$res:u/admin/gh' }
			} as any)
		).toBe('u/admin/gh')
	})

	it('names web search by a reserved name, whatever label it carries', () => {
		// It reaches the model as a provider capability rather than a tool, so the editor's label is
		// not a name: something else in the roster could carry it and be switched on with it.
		expect(toolEnabledName({ id: 'w', value: { tool_type: 'websearch' } } as any)).toBe(
			WEBSEARCH_ENABLED_NAME
		)
		expect(
			toolEnabledName({ id: 'w', summary: 'Web Search', value: { tool_type: 'websearch' } } as any)
		).toBe(WEBSEARCH_ENABLED_NAME)
	})

	it('reserves that name against every other kind', () => {
		// A flow module tool cannot be called it, so enabling a tool never enables web search beside
		// it. `getToolNameError` is the rule that holds, and the space is what stays outside it.
		expect(getToolNameError(WEBSEARCH_ENABLED_NAME)).toBeDefined()
	})
})
