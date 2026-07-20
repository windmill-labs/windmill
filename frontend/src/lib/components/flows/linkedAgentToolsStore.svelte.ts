import type { AgentTool } from './agentToolUtils'

// Tools resolved from a linked agent's `ai_agent` resource, keyed by the agent module id. A linked
// step stores `tools: []` (the tools live in the resource), so the graph and the tool editor read
// the resolved set from here. Populated at flow load (see flowState.ts, gated before first paint so
// nodes don't pop in) and refreshed when a step's link changes.
let toolsByModule = $state<Record<string, AgentTool[]>>({})

export function setLinkedAgentTools(moduleId: string, tools: AgentTool[]) {
	toolsByModule[moduleId] = tools
}

export function clearLinkedAgentTools(moduleId: string) {
	delete toolsByModule[moduleId]
}

export function getLinkedAgentTools(moduleId: string): AgentTool[] {
	return toolsByModule[moduleId] ?? []
}

/** Reactive snapshot of the whole map — read this where a computation must react to resolution. */
export function linkedAgentTools(): Record<string, AgentTool[]> {
	return toolsByModule
}
