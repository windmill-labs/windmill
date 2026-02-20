import type { AiAgent, FlowModule, FlowModuleValue, InputTransform } from '$lib/gen'
import { loadStoredConfig } from '../aiProviderStorage'
import { AI_AGENT_SCHEMA } from './flowInfers'

export const SPECIAL_TOOL_KINDS = ['mcpTool', 'websearchTool', 'aiAgentTool'] as const
export type SpecialToolKind = (typeof SPECIAL_TOOL_KINDS)[number]

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
	const input_transforms: AiAgent['input_transforms'] = {
		provider: {
			type: 'static',
			value: loadStoredConfig() ?? { kind: 'openai', resource: '', model: '' }
		},
		output_type: { type: 'static', value: 'text' },
		user_message: { type: 'ai' }
	}
	for (const key of Object.keys(AI_AGENT_SCHEMA.properties ?? {})) {
		if (!(key in input_transforms)) {
			;(input_transforms as Record<string, InputTransform>)[key] = {
				type: 'static',
				value: undefined
			}
		}
	}

	return {
		id,
		summary: '',
		value: {
			tool_type: 'flowmodule',
			type: 'aiagent',
			tools: [],
			input_transforms
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
 * Convert a FlowModuleTool to a FlowModule for use with loadFlowModuleState etc.
 * Strips the extra `tool_type` field and maps AgentTool fields to FlowModule fields.
 */
export function agentToolToFlowModule(tool: FlowModuleTool): FlowModule {
	const { tool_type: _, ...value } = tool.value
	return {
		id: tool.id,
		summary: tool.summary,
		value: value as FlowModuleValue
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
