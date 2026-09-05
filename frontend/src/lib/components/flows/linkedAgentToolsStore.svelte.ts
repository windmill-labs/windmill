import { deepEqual } from 'fast-equals'
import type { AgentTool } from './agentToolUtils'

// Tools resolved from a linked agent's `ai_agent` resource: the step stores `tools: []`, so the graph
// and the tool editor read the resolved set from here. Keyed by scope then module id — the scope
// keeps flows shown at the same time (an editor and an embedded preview) from aliasing each other.
let byScope = $state<Record<string, Record<string, AgentTool[]>>>({})

// Which agent each entry resolved from, so a write to one agent can be reflected on every step that
// links it rather than only the one that triggered the write. Plain, not reactive: it is read
// imperatively when reconciling a deploy, never rendered.
const agentRefByEntry = new Map<string, string>()

/** A step may name its agent bare or as `$res:<path>`; both are the same agent, and the index has
 *  to answer for a lookup written either way. */
function normalizeAgentRef(agentRef: string): string {
	return agentRef.replace(/^\$res:/, '').replace(/^res:\/\//, '')
}

function entryKey(scope: string, moduleId: string): string {
	return `${scope}\u0000${moduleId}`
}

function forgetScopeRefs(scope: string) {
	const prefix = `${scope}\u0000`
	for (const key of agentRefByEntry.keys()) {
		if (key.startsWith(prefix)) agentRefByEntry.delete(key)
	}
}

// A long-lived tab would otherwise keep every flow it ever visited (raw tool script contents
// included). Past this cap the least recently used scope is evicted, skipping any a mounted view
// still holds.
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

// Scopes a mounted view is currently relying on. A run viewer keeps one per nested job, hidden ones
// included, so the cap alone would evict buckets still in use — and restoring one would evict
// another, forever. Retained scopes are never evicted; the cap yields to correctness.
const retainedScopes = new Map<string, number>()
// A just-renamed scope, protected from eviction until a reader retains it: holders release the old
// key before retaining the new one, so the migrated bucket is unretained in between.
let pendingMigration: string | undefined = undefined

export function retainLinkedToolsScope(scope: string) {
	retainedScopes.set(scope, (retainedScopes.get(scope) ?? 0) + 1)
	if (scope === pendingMigration) {
		pendingMigration = undefined
	}
}

export function releaseLinkedToolsScope(scope: string) {
	const next = (retainedScopes.get(scope) ?? 0) - 1
	if (next > 0) {
		retainedScopes.set(scope, next)
	} else {
		retainedScopes.delete(scope)
	}
	// Scopes skipped while retained are otherwise never reconsidered, leaving the store over the cap
	// for the tab's life once enough views have been closed.
	evictOverCap()
}

// `protect` is the scope just written: it is the newest, so when every older one is retained it
// would otherwise be the only eligible victim and get dropped the moment it was published.
function evictOverCap(protect?: string) {
	while (scopeOrder.length > MAX_SCOPES) {
		const victim = scopeOrder.find(
			(s) => s !== protect && s !== pendingMigration && !retainedScopes.has(s)
		)
		if (victim === undefined) {
			break
		}
		scopeOrder = scopeOrder.filter((s) => s !== victim)
		delete byScope[victim]
		forgetScopeRefs(victim)
	}
}

function touchScope(scope: string) {
	scopeOrder = [...scopeOrder.filter((s) => s !== scope), scope]
	evictOverCap(scope)
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

export function setLinkedAgentTools(
	scope: string,
	moduleId: string,
	tools: AgentTool[],
	/** The agent these tools came from. Taken here rather than recorded separately so the index and
	 *  the tools cannot drift: every writer has the ref in hand, and one that forgot to file it left
	 *  a step invisible to `linkedModulesForAgent`. */
	agentRef: string
) {
	agentRefByEntry.set(entryKey(scope, moduleId), normalizeAgentRef(agentRef))
	// Publishers re-run and hand us a fresh-but-equal array each time; only mutate on a real change,
	// else the version bump would retrigger the graph recompute in a loop.
	if (deepEqual(byScope[scope]?.[moduleId], tools)) return
	byScope[scope] = { ...(byScope[scope] ?? {}), [moduleId]: tools }
	touchScope(scope)
	version++
}

/** Every module in this scope resolved from `agentRef`. A saved agent can be linked by more than
 *  one step, and all of them show tools that a write to it has just changed. */
export function linkedModulesForAgent(scope: string, agentRef: string): string[] {
	const prefix = `${scope}\u0000`
	const wanted = normalizeAgentRef(agentRef)
	const out: string[] = []
	for (const [key, ref] of agentRefByEntry) {
		if (key.startsWith(prefix) && ref === wanted) out.push(key.slice(prefix.length))
	}
	return out
}

/** Move one scope's resolutions into another: used when a rename moves readers to a new scope,
 * and to sweep republished data (keyed by the flow doc's path) into the live-edited scope. The
 * source bucket carries the newer resolution in both cases, so it wins the merge. */
export function migrateLinkedAgentToolsScope(oldScope: string, newScope: string) {
	if (oldScope === newScope || byScope[oldScope] === undefined) return
	byScope[newScope] = { ...(byScope[newScope] ?? {}), ...(byScope[oldScope] ?? {}) }
	for (const moduleId of Object.keys(byScope[oldScope] ?? {})) {
		const ref = agentRefByEntry.get(entryKey(oldScope, moduleId))
		if (ref !== undefined) agentRefByEntry.set(entryKey(newScope, moduleId), ref)
	}
	forgetScopeRefs(oldScope)
	delete byScope[oldScope]
	pendingMigration = newScope
	// Mark the new key most-recently-used but don't evict here: the next publish or release enforces
	// the cap, by which point readers have re-retained under the new key.
	scopeOrder = [...scopeOrder.filter((s) => s !== oldScope && s !== newScope), newScope]
	version++
}

export function clearLinkedAgentTools(scope: string, moduleId: string) {
	agentRefByEntry.delete(entryKey(scope, moduleId))
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
