import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { ResourceService } from '$lib/gen'
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import { isFlowModuleTool, agentToolToFlowModule, type AgentTool } from './agentToolUtils'
import { linkedToolsScope, setLinkedAgentTools } from './linkedAgentToolsStore.svelte'
import { clearAgentEditingForFlow } from './agentEditStore.svelte'
import { loadFlowModuleState } from './flowStateUtils.svelte'
import { emptyFlowModuleState } from './utils.svelte'
import type { StateStore } from '$lib/utils'

export type FlowModuleState = {
	schema?: Schema
	previewResult?: any
	previewArgs?: any
	previewJobId?: string
	previewSuccess?: boolean
	previewLogs?: string
}

export type FlowState = Record<string, FlowModuleState>

/**
 * flowStateStore represents the local state of each module indexed by its id.
 * It contains data loaded that are not contained in a Flow object i.e. schemas.
 * We also hold the data of the results of a test job, ran by the user.
 */

export async function initFlowState(
	flow: Flow,
	flowStateStore: StateStore<FlowState>,
	// The acting workspace when the flow editor runs in an AI session; else the nav workspace.
	workspace: string | undefined,
	// Flow path half of the linked-agent tools scope; keeps agents that share a module id across
	// simultaneously-shown flows from aliasing each other. Required so call sites can't silently
	// publish into the '' bucket while the graph reads the real flow path.
	flowPath: string
) {
	const modulesState: FlowState = {}

	const ws = workspace ?? get(workspaceStore)
	// The flow state is being (re)built wholesale (load, session-draft update): any pending
	// out-of-band agent Edit mode no longer corresponds to the incoming modules.
	clearAgentEditingForFlow(ws, flowPath)
	const scope = linkedToolsScope(ws, flowPath)
	await mapFlowModules(flow.value.modules, modulesState, workspace, scope)

	const failureModule = flow.value.failure_module
		? await loadFlowModuleState(flow.value.failure_module, workspace)
		: emptyFlowModuleState()

	flowStateStore.val = {
		...modulesState,
		failure: failureModule
	}
}

/**
 * mapFlowModule recursively explore the flow, following deeply nested loop and branches modules
 * to build the initial state.
 */
async function mapFlowModule(
	flowModule: FlowModule,
	modulesState: FlowState,
	workspace?: string,
	scope: string = ''
) {
	const value = flowModule.value
	if (value.type === 'forloopflow') {
		await mapFlowModules(value.modules, modulesState, workspace, scope)
	}

	if (value.type === 'branchone') {
		await mapFlowModules(value.default, modulesState, workspace, scope)
	}

	if (value.type === 'branchone' || value.type === 'branchall') {
		await Promise.all(
			value.branches.map(
				(branchModule: { summary?: string; skip_failure?: boolean; modules: Array<FlowModule> }) =>
					mapFlowModules(branchModule.modules, modulesState, workspace, scope)
			)
		)
	}

	if (value.type === 'aiagent') {
		const agentRef = (value as { agent?: string }).agent
		if (agentRef) {
			// A linked step's tools come from the resource (its own `tools` is empty); resolve them so
			// the graph can render its tool nodes. They are display-only (their inputs are edited in
			// the step panel, which infers schemas itself), so no per-tool module state is loaded —
			// resource tool ids are not flow-unique and must not key into the flow state.
			setLinkedAgentTools(scope, flowModule.id, await resolveLinkedAgentTools(agentRef, workspace))
		} else {
			await Promise.all(
				(value.tools ?? []).filter(isFlowModuleTool).map(async (tool) => {
					modulesState[tool.id] = await loadFlowModuleState(agentToolToFlowModule(tool), workspace)
				})
			)
		}
	}

	if (value.type === 'identity') {
		modulesState[flowModule.id] = emptyFlowModuleState()
	} else {
		const flowModuleState = await loadFlowModuleState(flowModule, workspace)
		modulesState[flowModule.id] = flowModuleState
	}
}

// Fetch a linked agent's tool set from its `ai_agent` resource. Degrades to no tools when the
// resource is missing or inaccessible so a broken link never stalls the flow load.
export async function resolveLinkedAgentTools(
	agentRef: string,
	workspace?: string
): Promise<AgentTool[]> {
	const ws = workspace ?? get(workspaceStore)
	if (!ws) return []
	const path = agentRef.replace(/^\$res:/, '').replace(/^res:\/\//, '')
	try {
		const res = await ResourceService.getResource({ workspace: ws, path })
		return ((res.value as { tools?: AgentTool[] } | undefined)?.tools ?? []) as AgentTool[]
	} catch {
		return []
	}
}

async function mapFlowModules(
	flowModules: FlowModule[],
	modulesState: FlowState,
	workspace?: string,
	scope: string = ''
) {
	await Promise.all(
		flowModules.map((flowModule: FlowModule) =>
			mapFlowModule(flowModule, modulesState, workspace, scope)
		)
	)
}
