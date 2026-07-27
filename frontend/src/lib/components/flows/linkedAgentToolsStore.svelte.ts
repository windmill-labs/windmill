import { deepEqual } from 'fast-equals'
import type { AgentTool } from './agentToolUtils'

// Tools resolved from a linked agent's `ai_agent` resource, keyed by a scope (the flow path) then
// the agent module id. A linked step stores `tools: []` (the tools live in the resource), so the
// graph and the tool editor read the resolved set from here. Populated at flow load (see
// flowState.ts, gated before first paint so nodes don't pop in) and refreshed when a step's link
// changes. The scope prevents agents that share a module id across different flows shown at the same
// time (e.g. an editor and an embedded flow preview) from aliasing each other's tools.
let byScope = $state<Record<string, Record<string, AgentTool[]>>>({})

// Nothing drops a scope when its flow stops being displayed, so a long-lived tab would otherwise
// retain every flow it ever visited (raw tool script contents included). Evict least-recently-
// published scopes past this cap, set far above the handful a single view can show at once so a
// still-displayed flow can never lose its tool nodes to eviction.
const MAX_SCOPES = 32
let scopeOrder: string[] = []

// Recency on read, so a scope still being displayed isn't evicted by unrelated publishes (a run
// viewer opens one bucket per nested job). Only reorders the plain recency list — evicting here
// would mutate reactive state during a render.
function noteScopeRead(scope: string) {
	if (byScope[scope] === undefined) return
	const last = scopeOrder[scopeOrder.length - 1]
	if (last === scope) return
	scopeOrder = [...scopeOrder.filter((s) => s !== scope), scope]
}

function touchScope(scope: string) {
	scopeOrder = [...scopeOrder.filter((s) => s !== scope), scope]
	while (scopeOrder.length > MAX_SCOPES) {
		delete byScope[scopeOrder.shift()!]
	}
}

/**
 * Scope key for the store: workspace + flow path. Flow paths repeat across workspaces, so a late
 * async resolution from a previous workspace must land in its own bucket instead of overwriting
 * the tools of an identically-named flow in the current one. Every publisher and reader must
 * derive the workspace the same way (operating workspace, falling back to the nav workspace).
 */
export function linkedToolsScope(
	workspace: string | undefined,
	flowPath: string | undefined
): string {
	return `${workspace ?? ''}:${flowPath ?? ''}`
}
// Bumped on every mutation. Non-reactive graph recomputations (which read the map inside untrack)
// track this to re-run when a link resolves after the initial render, e.g. right after linking.
let version = $state(0)

export function setLinkedAgentTools(scope: string, moduleId: string, tools: AgentTool[]) {
	// Publishers re-run and hand us a fresh-but-equal array each time; only mutate on a real change,
	// else the version bump would retrigger the graph recompute in a loop.
	if (deepEqual(byScope[scope]?.[moduleId], tools)) return
	byScope[scope] = { ...(byScope[scope] ?? {}), [moduleId]: tools }
	touchScope(scope)
	version++
}

/** Move one scope's resolutions into another: used when a rename moves readers to a new scope,
 * and to sweep republished data (keyed by the flow doc's path) into the live-edited scope. The
 * source bucket carries the newer resolution in both cases, so it wins the merge. */
export function migrateLinkedAgentToolsScope(oldScope: string, newScope: string) {
	if (oldScope === newScope || byScope[oldScope] === undefined) return
	byScope[newScope] = { ...(byScope[newScope] ?? {}), ...(byScope[oldScope] ?? {}) }
	delete byScope[oldScope]
	scopeOrder = scopeOrder.filter((s) => s !== oldScope)
	touchScope(newScope)
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
	noteScopeRead(scope)
	return byScope[scope]?.[moduleId] ?? []
}

/** Reactive snapshot of one scope's module→tools map — read this where a computation must react to
 * resolution (the graph passes it to computeAIToolNodes, which indexes it by module id). */
export function linkedAgentToolsForScope(scope: string): Record<string, AgentTool[]> {
	noteScopeRead(scope)
	return byScope[scope] ?? {}
}

/** Reactive counter that changes on any link resolution. Track it to trigger a recompute. */
export function linkedAgentToolsVersion(): number {
	return version
}
