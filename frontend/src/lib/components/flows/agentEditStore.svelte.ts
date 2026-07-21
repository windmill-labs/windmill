// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent),
// keyed by workspace + flow path + module id. Kept out of AgentResourceBar's local state because
// that component unmounts when another node is selected — the "Editing <path>" mode must survive
// navigating to a tool. Fully qualified key: module ids (a, b, …) repeat across flows and flow
// paths repeat across workspaces (fork/session editors), and a leaked entry would show a phantom
// Editing bar elsewhere whose Save could overwrite an unrelated agent.
let editingByStep = $state<Record<string, string | undefined>>({})

function key(workspace: string | undefined, flowPath: string, moduleId: string): string {
	return `${workspace ?? ''}:${flowPath}:${moduleId}`
}

export function getAgentEditingPath(
	workspace: string | undefined,
	flowPath: string,
	moduleId: string
): string | undefined {
	return editingByStep[key(workspace, flowPath, moduleId)]
}

export function setAgentEditingPath(
	workspace: string | undefined,
	flowPath: string,
	moduleId: string,
	path: string | undefined
) {
	editingByStep[key(workspace, flowPath, moduleId)] = path
}

/**
 * Invalidate all pending edit modes for a flow. Called when the flow's state is replaced wholesale
 * (undo/redo, session-draft updates, re-init): a stale "Editing" target no longer corresponds to
 * the restored modules, and saving it would overwrite the shared agent with unrelated config.
 */
export function clearAgentEditingForFlow(workspace: string | undefined, flowPath: string) {
	const prefix = `${workspace ?? ''}:${flowPath}:`
	for (const k of Object.keys(editingByStep)) {
		if (k.startsWith(prefix)) {
			delete editingByStep[k]
		}
	}
}

/**
 * Re-key pending edit modes when the flow is renamed in place, so an in-progress "Editing" fork
 * keeps its banner (and Save/Cancel) instead of being stranded under the old path.
 */
export function migrateAgentEditingFlow(
	workspace: string | undefined,
	oldPath: string,
	newPath: string
) {
	if (oldPath === newPath) return
	const oldPrefix = `${workspace ?? ''}:${oldPath}:`
	const newPrefix = `${workspace ?? ''}:${newPath}:`
	for (const k of Object.keys(editingByStep)) {
		if (k.startsWith(oldPrefix)) {
			editingByStep[newPrefix + k.slice(oldPrefix.length)] = editingByStep[k]
			delete editingByStep[k]
		}
	}
}
