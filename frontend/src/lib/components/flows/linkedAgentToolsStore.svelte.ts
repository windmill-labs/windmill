import { deepEqual } from 'fast-equals'
import type { AgentTool } from './agentToolUtils'

// Tools resolved from a linked agent's `ai_agent` resource, keyed by the agent module id. A linked
// step stores `tools: []` (the tools live in the resource), so the graph and the tool editor read
// the resolved set from here. Populated at flow load (see flowState.ts, gated before first paint so
// nodes don't pop in) and refreshed when a step's link changes.
let toolsByModule = $state<Record<string, AgentTool[]>>({})
// Bumped on every mutation. Non-reactive graph recomputations (which read the map inside untrack)
// track this to re-run when a link resolves after the initial render, e.g. right after linking.
let version = $state(0)

export function setLinkedAgentTools(moduleId: string, tools: AgentTool[]) {
	// Publishers re-run and hand us a fresh-but-equal array each time; only mutate on a real change,
	// else the version bump would retrigger the graph recompute in a loop.
	if (deepEqual(toolsByModule[moduleId], tools)) return
	toolsByModule[moduleId] = tools
	version++
}

export function clearLinkedAgentTools(moduleId: string) {
	if (!(moduleId in toolsByModule)) return
	delete toolsByModule[moduleId]
	version++
}

export function getLinkedAgentTools(moduleId: string): AgentTool[] {
	return toolsByModule[moduleId] ?? []
}

/** Reactive snapshot of the whole map — read this where a computation must react to resolution. */
export function linkedAgentTools(): Record<string, AgentTool[]> {
	return toolsByModule
}

/** Reactive counter that changes on any link resolution. Track it to trigger a recompute. */
export function linkedAgentToolsVersion(): number {
	return version
}
