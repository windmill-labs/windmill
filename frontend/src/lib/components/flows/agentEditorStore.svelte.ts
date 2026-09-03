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
	/** Where to re-resolve a graph's tool nodes after a deploy, when opened from a flow step. An
	 *  agent editor hosts its own flow under the agent's path, so `fromAgentEditor` is what tells
	 *  the two apart: a flow and a resource may share a path string, and `flowPath` alone would let
	 *  a step of one be read as a tool of the other. */
	host?: { flowPath: string; moduleId: string; fromAgentEditor?: boolean }
}

/**
 * Whether this target was opened from a step of the flow at `flowPath` — never from an agent
 * editor hosted there. That editor's host flow is named by the agent's own path, and a flow may
 * carry the same path string, so the flag is the only thing separating the two: without it a flow
 * mount claims a nested agent belonging to the editor, and both then build a whole editor over the
 * one draft.
 */
export function isStepOfFlow(t: AgentEditorTarget, flowPath: string | undefined): boolean {
	return !t.host?.fromAgentEditor && t.host?.flowPath === flowPath
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

/** How many times each agent has been written, so a surface that reads the resource can refetch it.
 *  The editor and the step card that opens it are separate components over one resource, neither
 *  owning the other, and a deploy that leaves the path alone changes nothing they otherwise key on. */
let agentWrites = $state<Record<string, number>>({})

function writeKey(workspace: string | undefined, path: string | undefined): string {
	return `${workspace ?? ''}:${path ?? ''}`
}

export function markAgentWritten(workspace: string | undefined, path: string) {
	const key = writeKey(workspace, path)
	agentWrites[key] = (agentWrites[key] ?? 0) + 1
}

export function agentWriteCount(workspace: string | undefined, path: string | undefined): number {
	return agentWrites[writeKey(workspace, path)] ?? 0
}
