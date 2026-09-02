/**
 * Which saved agent the agent editor is showing, and which of its tools.
 *
 * The editor edits the `ai_agent` resource through its own draft, so this holds a resource path
 * rather than a flow node: the same agent opens the same editor from a flow step and from the
 * resources page. Module-level rather than a context value because what opens it — a step's card,
 * a list row — unmounts the moment the selection moves.
 */
export interface AgentEditorTarget {
	path: string
	/** The workspace the opener operates on; the nav workspace when absent. */
	workspace?: string
	toolId?: string
	/** A level of the editor that is not the form. Mutually exclusive with `toolId`. */
	view?: 'evals'
	/** Where to re-resolve a graph's tool nodes after a deploy, when opened from a flow step. */
	host?: { flowPath: string; moduleId: string }
}

let target = $state<AgentEditorTarget | undefined>(undefined)

export function openAgentEditor(open: Omit<AgentEditorTarget, 'toolId' | 'view'>) {
	target = { ...open }
}

export function closeAgentEditor() {
	target = undefined
}

/** Navigate between the agent and one of its tools. Passing undefined returns to the agent. */
export function showAgentEditorTool(toolId: string | undefined) {
	if (!target) return
	target = { ...target, toolId, view: undefined }
}

/** Navigate between the agent and one of its own levels. Passing undefined returns to the agent. */
export function showAgentEditorView(view: AgentEditorTarget['view']) {
	if (!target) return
	target = { ...target, view, toolId: undefined }
}

export function agentEditorTarget(): AgentEditorTarget | undefined {
	return target
}
