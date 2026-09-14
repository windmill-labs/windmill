import { describe, expect, it, vi } from 'vitest'

// `agentToolUtils` reaches the copilot bundle, and Monaco's CSS with it, through this one import.
// Only `createAiAgentTool` reads it, and nothing below does.
vi.mock('../aiProviderStorage', () => ({ loadStoredConfig: () => undefined }))

import { toolEnabledName, WEBSEARCH_ENABLED_NAME } from './agentToolUtils'

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

	it('falls back to a constant for web search authored without a label', () => {
		// The editor always writes one and offers no way to clear it; JSON authored anywhere else may
		// carry none, and an entry with no name could not be enabled at all.
		expect(toolEnabledName({ id: 'w', value: { tool_type: 'websearch' } } as any)).toBe(
			WEBSEARCH_ENABLED_NAME
		)
		expect(
			toolEnabledName({ id: 'w', summary: 'Web Search', value: { tool_type: 'websearch' } } as any)
		).toBe('Web Search')
	})
})
