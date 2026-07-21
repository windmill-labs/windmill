// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent).
// Lives outside AgentResourceBar so the "Editing <path>" mode survives that component unmounting
// when another node is selected.
//
// Entries are looked up by the forked step's `tools` array identity, not by any location key:
// Edit assigns a fresh array (unique per fork, including nested editors reusing a module id),
// in-place edits keep it, and wholesale state replacement (undo, session drafts) yields new
// objects that simply never match — a stale entry can't resurface as a phantom Editing banner.
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
