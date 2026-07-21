// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent),
// keyed by flow path + module id. Kept out of AgentResourceBar's local state because that component
// unmounts when another node is selected — the "Editing <path>" mode must survive navigating to a
// tool. Keyed by flow path too: module ids (a, b, …) repeat across flows, and a leaked entry would
// otherwise show a phantom Editing bar in an unrelated flow whose Save could overwrite the agent.
let editingByStep = $state<Record<string, string | undefined>>({})

function key(flowPath: string, moduleId: string): string {
	return `${flowPath}:${moduleId}`
}

export function getAgentEditingPath(flowPath: string, moduleId: string): string | undefined {
	return editingByStep[key(flowPath, moduleId)]
}

export function setAgentEditingPath(flowPath: string, moduleId: string, path: string | undefined) {
	editingByStep[key(flowPath, moduleId)] = path
}
