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
	return tool.value.tool_type === 'flowmodule'
}

/**
 * Type guard to check if a tool is an MCP tool
 */
export function isMcpTool(tool: AgentTool): tool is McpTool {
	return tool.value.tool_type === 'mcp'
}

/**
 * Create a FlowModule tool from a FlowModuleValue
 */
export function createFlowModuleTool(
	id: string,
	summary: string | undefined,
	value: FlowModuleValue
): FlowModuleTool {
	return {
		id,
		summary,
		value: {
			tool_type: 'flowmodule',
			...value
		} as FlowModuleTool['value']
	}
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
 * Get display name for a tool
 */
export function getToolDisplayName(tool: AgentTool): string {
	if (tool.summary) {
		return tool.summary
	}
	if (isMcpTool(tool)) {
		return `MCP: ${tool.value.resource_path}`
	}
	if (isFlowModuleTool(tool)) {
		if (tool.value.type === 'rawscript') {
			return tool.id
		}
		if (tool.value.type === 'script') {
			return tool.value.path ?? tool.id
		}
		if (tool.value.type === 'flow') {
			return tool.value.path ?? tool.id
		}
	}
	return tool.id
}

/**
 * Get the FlowModuleValue from a tool (only for FlowModule tools)
 */
export function getFlowModuleValue(tool: AgentTool): FlowModuleValue | undefined {
	if (isFlowModuleTool(tool)) {
		const { tool_type, ...rest } = tool.value
		return rest as FlowModuleValue
	}
	return undefined
}

/**
 * Convert an AgentTool to a FlowModule for use with existing components
 * This allows reusing existing FlowModule editing components
 */
export function agentToolToFlowModule(tool: AgentTool): FlowModule | undefined {
	if (isFlowModuleTool(tool)) {
		const { tool_type, ...flowModuleValue } = tool.value
		return {
			id: tool.id,
			summary: tool.summary,
			value: flowModuleValue as FlowModuleValue
		}
	}
	// MCP tools cannot be converted to FlowModule since mcpserver type no longer exists
	return undefined
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
