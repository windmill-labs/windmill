import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { isFlowModuleTool, agentToolToFlowModule } from './agentToolUtils'
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
	workspace?: string
) {
	const modulesState: FlowState = {}

	await mapFlowModules(flow.value.modules, modulesState, workspace)

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
async function mapFlowModule(flowModule: FlowModule, modulesState: FlowState, workspace?: string) {
	const value = flowModule.value
	if (value.type === 'forloopflow') {
		await mapFlowModules(value.modules, modulesState, workspace)
	}

	if (value.type === 'branchone') {
		await mapFlowModules(value.default, modulesState, workspace)
	}

	if (value.type === 'branchone' || value.type === 'branchall') {
		await Promise.all(
			value.branches.map(
				(branchModule: { summary?: string; skip_failure?: boolean; modules: Array<FlowModule> }) =>
					mapFlowModules(branchModule.modules, modulesState, workspace)
			)
		)
	}

	if (value.type === 'aiagent' && value.tools) {
		await Promise.all(
			value.tools.filter(isFlowModuleTool).map(async (tool) => {
				modulesState[tool.id] = await loadFlowModuleState(agentToolToFlowModule(tool), workspace)
			})
		)
	}

	if (value.type === 'identity') {
		modulesState[flowModule.id] = emptyFlowModuleState()
	} else {
		const flowModuleState = await loadFlowModuleState(flowModule, workspace)
		modulesState[flowModule.id] = flowModuleState
	}
}

async function mapFlowModules(
	flowModules: FlowModule[],
	modulesState: FlowState,
	workspace?: string
) {
	await Promise.all(
		flowModules.map((flowModule: FlowModule) => mapFlowModule(flowModule, modulesState, workspace))
	)
}
