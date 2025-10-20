import type { AiAgent, FlowModule, FlowModuleValue } from '$lib/gen'

// Type aliases for better readability
export type AgentTool = AiAgent['tools'][number]
export type FlowModuleTool = AgentTool & { value: { tool_type: 'flowmodule' } & FlowModuleValue }
export type McpTool = AgentTool & {
	value: {
		tool_type: 'mcp'
		resource_path: string
		include_tools?: string[]
		exclude_tools?: string[]
	}
}

/**
 * Type guard to check if a tool is a FlowModule tool
 */
export function isFlowModuleTool(tool: AgentTool): tool is FlowModuleTool {
	return tool.value.tool_type === undefined || tool.value.tool_type === 'flowmodule'
}

/**
 * Type guard to check if a tool is an MCP tool
 */
export function isMcpTool(tool: AgentTool): tool is McpTool {
	return tool.value.tool_type === 'mcp'
}

/**
 * Create an MCP tool from resource path
 */
export function createMcpTool(id: string): McpTool {
	return {
		id,
		summary: '',
		value: {
			tool_type: 'mcp',
			resource_path: '',
			include_tools: [],
			exclude_tools: []
		}
	}
}

/**
 * Convert a FlowModule back to an AgentTool
 * Used when saving changes back to the AI Agent tools array
 */
export function flowModuleToAgentTool(flowModule: FlowModule): AgentTool {
	return {
		id: flowModule.id,
		summary: flowModule.summary,
		value: {
			tool_type: 'flowmodule',
			...flowModule.value
		} as FlowModuleTool['value']
	}
}
