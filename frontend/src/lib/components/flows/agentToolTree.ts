import type { FlowModule } from '$lib/gen'
import {
	getToolNameError,
	isFlowModuleTool,
	type AgentTool,
	type FlowModuleTool
} from './agentToolUtils'
import { forEachAiAgentModule } from './aiAgentModules'

type FlowNodeLike = Pick<FlowModule, 'id' | 'value'>

export type AgentToolOwner = {
	agentId: string
	tools: AgentTool[]
	toolIndex: number
	tool: AgentTool
	depth: number
}

export type RemovedAgentTool = {
	tool: AgentTool
	removedIds: string[]
}

export function findAgentToolOwner(
	modules: FlowModule[],
	toolId: string
): AgentToolOwner | undefined {
	return findAgentToolOwnerInModules(modules, toolId, 0)
}

export function removeAgentToolOwner(owner: AgentToolOwner): RemovedAgentTool | undefined {
	const candidate = owner.tools[owner.toolIndex]
	if (!candidate || candidate.id !== owner.tool.id) {
		return undefined
	}

	owner.tools.splice(owner.toolIndex, 1)
	return {
		tool: candidate,
		removedIds: collectAgentToolIds(candidate)
	}
}

export function collectFlowNodeIds(module: FlowModule): string[] {
	return collectFlowNodeIdsFromNode(module)
}

export function collectAgentToolIds(tool: AgentTool): string[] {
	return collectFlowNodeIdsFromNode(tool as FlowModuleTool)
}

function findAgentToolOwnerInModules(
	modules: FlowModule[],
	toolId: string,
	depth: number
): AgentToolOwner | undefined {
	for (const module of modules) {
		const owner = findAgentToolOwnerInNode(module, toolId, depth)
		if (owner) {
			return owner
		}
	}

	return undefined
}

function findAgentToolOwnerInNode(
	node: FlowNodeLike,
	toolId: string,
	depth: number
): AgentToolOwner | undefined {
	if (node.value.type === 'forloopflow' || node.value.type === 'whileloopflow') {
		return findAgentToolOwnerInModules(node.value.modules, toolId, depth)
	}

	if (node.value.type === 'branchall') {
		for (const branch of node.value.branches) {
			const owner = findAgentToolOwnerInModules(branch.modules, toolId, depth)
			if (owner) {
				return owner
			}
		}
		return undefined
	}

	if (node.value.type === 'branchone') {
		const defaultOwner = findAgentToolOwnerInModules(node.value.default, toolId, depth)
		if (defaultOwner) {
			return defaultOwner
		}
		for (const branch of node.value.branches) {
			const owner = findAgentToolOwnerInModules(branch.modules, toolId, depth)
			if (owner) {
				return owner
			}
		}
		return undefined
	}

	if (node.value.type !== 'aiagent') {
		return undefined
	}

	// Absent for a linked agent, whose tools live in the resource rather than on the module.
	const tools = node.value.tools ?? []
	const toolIndex = tools.findIndex((tool) => tool.id === toolId)
	if (toolIndex !== -1) {
		return {
			agentId: node.id,
			tools,
			toolIndex,
			tool: tools[toolIndex],
			depth: depth + 1
		}
	}

	for (const tool of tools) {
		if (!isFlowModuleTool(tool)) {
			continue
		}

		const owner = findAgentToolOwnerInNode(tool as FlowNodeLike, toolId, depth + 1)
		if (owner) {
			return owner
		}
	}

	return undefined
}

function collectFlowNodeIdsFromNode(node: FlowNodeLike): string[] {
	const ids = [node.id]

	if (node.value.type === 'forloopflow' || node.value.type === 'whileloopflow') {
		for (const module of node.value.modules) {
			ids.push(...collectFlowNodeIds(module))
		}
		return ids
	}

	if (node.value.type === 'branchall') {
		for (const branch of node.value.branches) {
			for (const module of branch.modules) {
				ids.push(...collectFlowNodeIds(module))
			}
		}
		return ids
	}

	if (node.value.type === 'branchone') {
		for (const module of node.value.default) {
			ids.push(...collectFlowNodeIds(module))
		}
		for (const branch of node.value.branches) {
			for (const module of branch.modules) {
				ids.push(...collectFlowNodeIds(module))
			}
		}
		return ids
	}

	if (node.value.type === 'aiagent') {
		for (const tool of node.value.tools ?? []) {
			ids.push(...collectAgentToolIds(tool))
		}
	}

	return ids
}

/** Ids of AI agent modules that neither link to a saved agent nor set a provider — they pass schema
 * validation (a linked step legitimately has no provider of its own) but fail on every run. */
export function collectProviderlessAgentIds(modules: unknown): string[] {
	const ids: string[] = []
	forEachAiAgentModule(modules, (mod, v) => {
		if (!v.agent && !v.input_transforms?.provider) {
			ids.push(mod.id)
		}
	})
	return ids
}

export type InvalidAgentToolName = {
	agentId: string
	toolId: string
	name: string
	error: string
}

/** Agent tools whose name (their `summary`) the worker rejects — the flow saves fine but every run
 * of the agent step fails, so writers must catch this before the flow is stored. */
export function collectInvalidAgentToolNames(modules: unknown): InvalidAgentToolName[] {
	const invalid: InvalidAgentToolName[] = []
	forEachAiAgentModule(modules, (mod, v) => {
		const tools: AgentTool[] = Array.isArray(v.tools) ? v.tools : []
		// Only a flowmodule tool's summary is a callable name: the worker never reads an mcp or
		// websearch summary, so a blank one there must stay writable (both default to '').
		const named = tools.filter(
			(tool) => tool.value?.tool_type !== 'mcp' && tool.value?.tool_type !== 'websearch'
		)
		const siblingNames = named.map((tool) => tool.summary ?? '')
		for (const tool of named) {
			const name = tool.summary ?? ''
			const error = getToolNameError(name, tool.value?.tool_type, siblingNames)
			if (error) {
				invalid.push({ agentId: mod.id, toolId: tool.id, name, error })
			}
		}
	})
	return invalid
}
