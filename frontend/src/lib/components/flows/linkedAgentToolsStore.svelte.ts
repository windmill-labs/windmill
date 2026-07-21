import { deepEqual } from 'fast-equals'
import type { AgentTool } from './agentToolUtils'

// Tools resolved from a linked agent's `ai_agent` resource, keyed by a scope (the flow path) then
// the agent module id. A linked step stores `tools: []` (the tools live in the resource), so the
// graph and the tool editor read the resolved set from here. Populated at flow load (see
// flowState.ts, gated before first paint so nodes don't pop in) and refreshed when a step's link
// changes. The scope prevents agents that share a module id across different flows shown at the same
// time (e.g. an editor and an embedded flow preview) from aliasing each other's tools.
let byScope = $state<Record<string, Record<string, AgentTool[]>>>({})
// Bumped on every mutation. Non-reactive graph recomputations (which read the map inside untrack)
// track this to re-run when a link resolves after the initial render, e.g. right after linking.
let version = $state(0)

export function setLinkedAgentTools(scope: string, moduleId: string, tools: AgentTool[]) {
	// Publishers re-run and hand us a fresh-but-equal array each time; only mutate on a real change,
	// else the version bump would retrigger the graph recompute in a loop.
	if (deepEqual(byScope[scope]?.[moduleId], tools)) return
	byScope[scope] = { ...(byScope[scope] ?? {}), [moduleId]: tools }
	version++
}

export function clearLinkedAgentTools(scope: string, moduleId: string) {
	if (byScope[scope]?.[moduleId] === undefined) return
	const rest = { ...byScope[scope] }
	delete rest[moduleId]
	byScope[scope] = rest
	version++
}

export function getLinkedAgentTools(scope: string, moduleId: string): AgentTool[] {
	return byScope[scope]?.[moduleId] ?? []
}

/** Reactive snapshot of one scope's module→tools map — read this where a computation must react to
 * resolution (the graph passes it to computeAIToolNodes, which indexes it by module id). */
export function linkedAgentToolsForScope(scope: string): Record<string, AgentTool[]> {
	return byScope[scope] ?? {}
}

/** Reactive counter that changes on any link resolution. Track it to trigger a recompute. */
export function linkedAgentToolsVersion(): number {
	return version
}
