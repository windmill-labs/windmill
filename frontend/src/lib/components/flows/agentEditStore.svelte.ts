// The path an AI agent step is being edited-in-place against (set by "Edit" on a linked agent),
// keyed by module id. Kept out of AgentResourceBar's local state because that component unmounts
// when a tool node is selected — the "Editing <path>" mode must survive navigating to a tool.
let editingByModule = $state<Record<string, string | undefined>>({})

export function getAgentEditingPath(moduleId: string): string | undefined {
	return editingByModule[moduleId]
}

export function setAgentEditingPath(moduleId: string, path: string | undefined) {
	editingByModule[moduleId] = path
}
