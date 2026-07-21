// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent).
// Lives outside AgentResourceBar so the "Editing <path>" mode survives that component unmounting
// when another node is selected.
//
// Entries are looked up by the forked step's `tools` array identity, not by any location key:
// Edit assigns a fresh array (unique per fork, including nested editors reusing a module id),
// in-place edits keep it, and wholesale state replacement (undo, session drafts) yields new
// objects that simply never match — a stale entry can't resurface as a phantom Editing banner.
import type { FlowModule, OpenFlow } from '$lib/gen'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'

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

// Refresh the flow store without dropping an in-progress agent Editing session: use at every
// content-preserving refresh site (structural edits, schema/failure/mock changes) where a bare
// refreshStateStore clone would break the marker identity above. Content-changing replacements
// (undo, YAML/diff/AI apply, session restores) stay bare so stale edit state keeps invalidating.
export function refreshFlowStateStore(flowStore: StateStore<OpenFlow>) {
	reanchorAgentEditsAcross(
		() => flowStore.val.value?.modules,
		() => refreshStateStore(flowStore)
	)
}

// Carry edit state across a wholesale clone of the flow: the clone replaces every `tools` array
// (breaking marker identity) while preserving content verbatim, so re-keying entries to the
// cloned arrays by module id is safe there — unlike undo or session restores, which change
// content and must keep invalidating.
export function reanchorAgentEditsAcross(
	getModules: () => FlowModule[] | undefined,
	refresh: () => void
) {
	const anchors: { key: string; path: string; marker: object }[] = []
	forEachAgentFork(getModules(), (key, tools) => {
		const path = getAgentEditingPath(tools)
		if (path) anchors.push({ key, path, marker: tools })
	})
	refresh()
	if (anchors.length === 0) return
	const byKey = new Map<string, object>()
	forEachAgentFork(getModules(), (key, tools) => byKey.set(key, tools))
	for (const a of anchors) {
		setAgentEditingPath(a.marker, undefined)
		const tools = byKey.get(a.key)
		if (tools) setAgentEditingPath(tools, a.path)
	}
}

// Visit every standalone (unlinked) agent's tools array, including nested agent tools. Keys are
// ancestry-qualified (`hostId/toolId`): resource-imported tool ids are not flow-global, so a
// nested agent may share a bare id with a flow module and must not alias it.
function forEachAgentFork(
	modules: FlowModule[] | undefined,
	visit: (key: string, tools: object) => void,
	prefix: string = ''
) {
	for (const mod of modules ?? []) {
		const v = mod.value as FlowModule['value'] | undefined
		if (!v) continue
		if (v.type === 'forloopflow' || v.type === 'whileloopflow') {
			forEachAgentFork(v.modules, visit, prefix)
		} else if (v.type === 'branchone') {
			forEachAgentFork(v.default, visit, prefix)
			for (const b of v.branches ?? []) forEachAgentFork(b.modules, visit, prefix)
		} else if (v.type === 'branchall') {
			for (const b of v.branches ?? []) forEachAgentFork(b.modules, visit, prefix)
		} else if (v.type === 'aiagent') {
			const key = `${prefix}${mod.id}`
			if (!v.agent && v.tools) {
				visit(key, v.tools)
			}
			forEachAgentFork(v.tools as FlowModule[] | undefined, visit, `${key}/`)
		}
	}
}
