import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { ResourceService } from '$lib/gen'
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import { isFlowModuleTool, agentToolToFlowModule, type AgentTool } from './agentToolUtils'
import { linkedToolsScope, setLinkedAgentTools } from './linkedAgentToolsStore.svelte'
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

// Latest linked-tool fetch per (scope, agent module); see the publish guard below.
const linkedToolFetchGen = new Map<string, number>()

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
	if (value.type === 'forloopflow' || value.type === 'whileloopflow') {
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
			await publishLinkedAgentTools(agentRef, workspace, scope, flowModule.id)
		} else {
			// Shape-checked because `tools` is JSON-authored: throwing here would skip the agent's
			// own state below, leaving it with no schema rather than with no tool schemas.
			const tools = Array.isArray(value.tools) ? value.tools : []
			await Promise.all(
				tools.filter(isFlowModuleTool).map(async (tool) => {
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

// Resolve a linked agent's tools and publish them into the store. Concurrent re-resolutions of the
// same (scope, module) race — un-awaited re-inits from session-draft sync, or a link swapped while
// the previous fetch is in flight — so only the latest may publish, else a superseded agent's tools
// overwrite the newer one.
export async function publishLinkedAgentTools(
	agentRef: string,
	workspace: string | undefined,
	scope: string,
	moduleId: string
) {
	const genKey = `${scope}:${moduleId}`
	const gen = claimLinkedToolsFetch(scope, moduleId)
	const tools = await resolveLinkedAgentTools(agentRef, workspace)
	if (linkedToolFetchGen.get(genKey) === gen) {
		setLinkedAgentTools(scope, moduleId, tools, agentRef)
	}
}

/** Supersede every in-flight fetch for a scope. A rename moves the bucket to a new key, so a fetch
 * still running against the old one would publish there and then be swept forward over tools that
 * resolved later under the new key. */
export function invalidateLinkedToolsFetches(scope: string) {
	const prefix = `${scope}:`
	for (const key of linkedToolFetchGen.keys()) {
		if (key.startsWith(prefix)) {
			linkedToolFetchGen.set(key, (linkedToolFetchGen.get(key) ?? 0) + 1)
		}
	}
}

/** Supersede any in-flight fetch for this (scope, module) and return the new generation. Anything
 * that publishes or clears tools outside `publishLinkedAgentTools` must claim first, or an older
 * fetch still passes its own check and overwrites the newer result. */
export function claimLinkedToolsFetch(scope: string, moduleId: string): number {
	const genKey = `${scope}:${moduleId}`
	const gen = (linkedToolFetchGen.get(genKey) ?? 0) + 1
	linkedToolFetchGen.set(genKey, gen)
	return gen
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
