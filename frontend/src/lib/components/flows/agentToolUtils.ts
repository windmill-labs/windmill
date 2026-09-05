import type { AiAgent, FlowModule, FlowModuleValue, InputTransform } from '$lib/gen'
import { loadStoredConfig } from '../aiProviderStorage'
import { AI_AGENT_SCHEMA } from './flowInfers'
import { forbiddenIds } from './idUtils'

/**
 * A tool's `summary` is the name the LLM sees, and the worker rejects any name that does not match
 * `^[a-zA-Z0-9_]+$` (`ai_executor.rs`), so an unvalidated name fails on every run of the flow.
 *
 * `kind` only has to tell the three tool kinds apart, so callers may pass either the raw
 * `value.tool_type` or the module type they resolved it to: anything other than `'mcp'` and
 * `'websearch'` — including `undefined` on a legacy tool — is checked as a flow module tool, which
 * is what the worker does too.
 */
export function getToolNameError(
	name: string,
	kind?: 'mcp' | 'websearch' | (string & {}),
	siblingNames?: string[]
): string | undefined {
	if (kind === 'websearch') return undefined
	if (kind === 'mcp') {
		return name.length > 0 ? undefined : 'Tool name must not be empty'
	}
	// Ahead of the pattern, which an empty name also fails: "must only contain letters" reads as a
	// complaint about characters that are not there.
	if (name.length === 0) {
		return 'Tool name must not be empty'
	}
	if (!/^[a-zA-Z0-9_]+$/.test(name)) {
		return 'Tool name must only contain letters, numbers and underscores'
	}
	if (forbiddenIds.includes(name)) {
		return `'${name}' is a reserved name`
	}
	if (siblingNames && siblingNames.filter((n) => n === name).length > 1) {
		return 'Duplicate tool name'
	}
	return undefined
}

export const SPECIAL_TOOL_KINDS = ['mcpTool', 'websearchTool', 'aiAgentTool'] as const
export type SpecialToolKind = (typeof SPECIAL_TOOL_KINDS)[number]

// Type aliases for better readability
export type AgentTool = NonNullable<AiAgent['tools']>[number]
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
	// `value` must be there, not merely lack a `tool_type`: the tool list is JSON-authored, and an
	// entry without one is not a flowmodule tool for the callers that go on to read its script.
	if (tool?.value == undefined) return false
	return tool.value.tool_type === undefined || tool.value.tool_type === 'flowmodule'
}

/**
 * Type guard to check if a tool is an MCP tool
 */
export function isMcpTool(tool: AgentTool): tool is McpTool {
	return tool?.value?.tool_type === 'mcp'
}

/**
 * Type guard to check if a tool is a Websearch tool
 */
export function isWebsearchTool(tool: AgentTool): tool is WebsearchTool {
	return tool?.value?.tool_type === 'websearch'
}

/** The only input a nested agent used as a tool has the calling agent fill: the rest is its own
 *  configuration, not something to generate. Mirrors the server, which offers such a tool a schema
 *  of `{user_message}` and nothing else (`AI_AGENT_TOOL_SCHEMA` in `ai_executor.rs`); anything else
 *  left AI-filled here is dropped from that schema and never reaches the model. */
export const AI_AGENT_TOOL_AI_KEYS = ['user_message']

/** What a tool is called wherever it is named: its own name, else what it points at. Never its id,
 *  which is internal. Undefined when it has nothing to be called yet — an MCP tool with no server
 *  picked is unnamed rather than misnamed, so each surface words that for itself. */
export function toolDisplayName(tool: AgentTool): string | undefined {
	const value = tool?.value as Record<string, any>
	return tool?.summary || value?.path || value?.resource_path || undefined
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
 * Wrap a newly created FlowModule as an AgentTool.
 *
 * Only valid for a module that is not already a tool: FlowModule carries none of the
 * AgentTool-level metadata (`description`), so folding an edited module back into an
 * existing tool through here would drop it — spread over the existing tool instead.
 */
export function newFlowModuleAgentTool(flowModule: FlowModule): AgentTool {
	return {
		id: flowModule.id,
		summary: flowModule.summary,
		value: {
			tool_type: 'flowmodule',
			...flowModule.value
		} as FlowModuleTool['value']
	}
}
