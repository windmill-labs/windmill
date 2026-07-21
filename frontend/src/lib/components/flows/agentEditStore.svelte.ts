// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent).
// Kept out of AgentResourceBar's local state because that component unmounts when another node is
// selected — the "Editing <path>" mode must survive navigating away and back.
//
// Validity is anchored to the forked step's `tools` array identity rather than a flow-path key:
// Edit assigns a fresh array, in-place edits keep it, and any wholesale state replacement (undo,
// redo, session-draft updates, opening a different flow that reuses the module id) produces new
// objects. A stale entry therefore fails the marker check instead of resurfacing a phantom
// Editing banner whose Save could overwrite the shared agent — while an in-place flow rename
// (which touches no modules) keeps the banner alive.
type Entry = { path: string; marker: object }

let editingByStep = $state<Record<string, Entry | undefined>>({})

function key(workspace: string | undefined, moduleId: string): string {
	return `${workspace ?? ''}:${moduleId}`
}

export function getAgentEditingPath(
	workspace: string | undefined,
	moduleId: string,
	marker: object | undefined
): string | undefined {
	const entry = editingByStep[key(workspace, moduleId)]
	return entry && marker && entry.marker === marker ? entry.path : undefined
}

export function setAgentEditingPath(
	workspace: string | undefined,
	moduleId: string,
	path: string | undefined,
	marker: object | undefined
) {
	editingByStep[key(workspace, moduleId)] = path && marker ? { path, marker } : undefined
}
