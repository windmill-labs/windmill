import type { AiAgent, FlowModule, FlowModuleValue } from '$lib/gen'
import { loadStoredConfig } from '../aiProviderStorage'

// Type aliases for better readability
export type AgentTool = AiAgent['tools'][number]
export type FlowModuleTool = AgentTool & { value: { tool_type: 'flowmodule' } & FlowModuleValue }
export type AiAgentTool = AgentTool & {
	value: { tool_type: 'flowmodule' } & { type: 'aiagent' } & FlowModuleValue
}
export type McpTool = AgentTool & {
	value: {
		tool_type: 'mcp'
		resource_path: string
		include_tools?: string[]
		exclude_tools?: string[]
	}
}
export type WebsearchTool = AgentTool & {
	value: {
		tool_type: 'websearch'
	}
}

/**
 * Type guard to check if a tool is an AI Agent tool (nested agent)
 */
export function isAiAgentTool(tool: AgentTool): tool is AiAgentTool {
	return isFlowModuleTool(tool) && tool.value.type === 'aiagent'
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
 * Type guard to check if a tool is a Websearch tool
 */
export function isWebsearchTool(tool: AgentTool): tool is WebsearchTool {
	return tool.value.tool_type === 'websearch'
}

/**
 * Create an AI Agent tool (nested agent)
 */
export function createAiAgentTool(id: string): AiAgentTool {
	return {
		id,
		summary: '',
		value: {
			tool_type: 'flowmodule',
			type: 'aiagent',
			tools: [],
			input_transforms: {
				provider: {
					type: 'static',
					value: loadStoredConfig() ?? { kind: 'openai', resource: '', model: '' }
				},
				output_type: { type: 'static', value: 'text' },
				user_message: { type: 'ai' }
			}
		}
	} as AiAgentTool
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
 * Create a Websearch tool
 */
export function createWebsearchTool(id: string): WebsearchTool {
	return {
		id,
		summary: 'Web Search',
		value: {
			tool_type: 'websearch'
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
