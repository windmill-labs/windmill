/**
 * Which agent step the agent editor is showing, and which of its tools.
 *
 * Module-level rather than a context value: the modal is mounted where `FlowEditorContext` is set,
 * while what opens it is the step's own card, which unmounts the moment the selection moves. The
 * editor is a view onto the step's existing fork-for-edit session, so this holds only where to
 * look, never the edits themselves.
 */
let target = $state<{ agentId: string; toolId?: string } | undefined>(undefined)

export function openAgentEditor(agentId: string) {
	target = { agentId }
}

export function closeAgentEditor() {
	target = undefined
}

/** Navigate between the agent and one of its tools. Passing undefined returns to the agent. */
export function showAgentEditorTool(toolId: string | undefined) {
	if (!target) return
	target = { agentId: target.agentId, toolId }
}

export function agentEditorTarget(): { agentId: string; toolId?: string } | undefined {
	return target
}

/** Whether the editor currently owns this step, so the step panel can stand down rather than mount
 *  a second copy of the same form, test runner and edit card. */
export function agentEditorOwns(moduleId: string | undefined): boolean {
	return moduleId !== undefined && target?.agentId === moduleId
}
