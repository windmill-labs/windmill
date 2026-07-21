// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent).
// Lives outside AgentResourceBar so the "Editing <path>" mode survives that component unmounting
// when another node is selected.
//
// Entries are looked up by the forked step's `tools` array identity, not by any location key:
// Edit assigns a fresh array (unique per fork, including nested editors reusing a module id),
// in-place edits keep it, and wholesale state replacement (undo, session drafts) yields new
// objects that simply never match — a stale entry can't resurface as a phantom Editing banner.
import type { FlowModule } from '$lib/gen'
import { forEachFlowModule } from './dfs'

type Entry = { path: string; marker: object }

// Capped: abandoned forks (editor closed without Save/Cancel) leave dead entries behind.
const MAX_ENTRIES = 20

let entries = $state<Entry[]>([])

export function getAgentEditingPath(marker: object | undefined): string | undefined {
	return marker ? entries.find((e) => e.marker === marker)?.path : undefined
}

export function setAgentEditingPath(marker: object | undefined, path: string | undefined) {
	if (!marker) return
	const rest = entries.filter((e) => e.marker !== marker)
	entries = path ? [...rest.slice(-(MAX_ENTRIES - 1)), { marker, path }] : rest
}

// Carry edit state across a wholesale clone of the flow: refreshStateStore deep-snapshots the
// store, replacing every `tools` array (breaking marker identity) while preserving content
// verbatim, so re-keying entries to the cloned arrays by module id is safe there — unlike undo
// or session restores, which change content and must keep invalidating.
export function reanchorAgentEditsAcross(
	getModules: () => FlowModule[] | undefined,
	refresh: () => void
) {
	const anchors: { id: string; path: string; marker: object }[] = []
	forEachAgentFork(getModules(), (id, tools) => {
		const path = getAgentEditingPath(tools)
		if (path) anchors.push({ id, path, marker: tools })
	})
	refresh()
	if (anchors.length === 0) return
	const byId = new Map<string, object>()
	forEachAgentFork(getModules(), (id, tools) => byId.set(id, tools))
	for (const a of anchors) {
		setAgentEditingPath(a.marker, undefined)
		const tools = byId.get(a.id)
		if (tools) setAgentEditingPath(tools, a.path)
	}
}

// Visit every standalone (unlinked) agent's tools array, including nested agent tools.
function forEachAgentFork(
	modules: FlowModule[] | undefined,
	visit: (id: string, tools: object) => void
) {
	forEachFlowModule(modules ?? [], (mod) => {
		const v = mod.value
		if (v.type === 'aiagent' && !v.agent && v.tools) {
			visit(mod.id, v.tools)
		}
	})
}
