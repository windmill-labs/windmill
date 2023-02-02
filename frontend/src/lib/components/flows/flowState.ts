import type { Schema } from '$lib/common'
import type { Flow, FlowModule, FlowStatusModule } from '$lib/gen'
import { writable } from 'svelte/store'
import { loadFlowModuleState } from './flowStateUtils'
import { emptyFlowModuleState } from './utils'

export type FlowModuleState = {
	schema: Schema
	previewResult?: any
	previewArgs?: any
}

export type FlowState = Record<string, FlowModuleState>

/**
 * flowStateStore represents the local state of each module indexed by its id.
 * It contains data loaded that are not contained in a Flow object i.e. schemas.
 * We also hold the data of the results of a test job, ran by the user.
 */
export const flowStateStore = writable<FlowState>({})
export const testStepStore = writable<Record<string, any>>({})

export async function initFlowState(flow: Flow) {
	const modulesState: FlowState = {}

	await mapFlowModules(flow.value.modules, modulesState)

	const failureModule = flow.value.failure_module
		? await loadFlowModuleState(flow.value.failure_module)
		: emptyFlowModuleState()

	flowStateStore.set({
		...modulesState,
		failure: failureModule
	})
}

/**
 * mapFlowModule recursively explore the flow, following deeply nested loop and branches modules
 * to build the initial state.
 */
async function mapFlowModule(flowModule: FlowModule, modulesState: FlowState) {
	const value = flowModule.value
	if (value.type === 'forloopflow') {
		await mapFlowModules(value.modules, modulesState)
	}

	if (value.type === 'branchone') {
		await mapFlowModules(value.default, modulesState)
	}

	if (value.type === 'branchone' || value.type === 'branchall') {
		await Promise.all(
			value.branches.map(
				(branchModule: { summary?: string; skip_failure?: boolean; modules: Array<FlowModule> }) =>
					mapFlowModules(branchModule.modules, modulesState)
			)
		)
	}

	if (value.type === 'identity') {
		modulesState[flowModule.id] = emptyFlowModuleState()
	} else {
		const flowModuleState = await loadFlowModuleState(flowModule)
		modulesState[flowModule.id] = flowModuleState
	}

}

async function mapFlowModules(flowModules: FlowModule[], modulesState: FlowState) {
	await Promise.all(
		flowModules.map((flowModule: FlowModule) => mapFlowModule(flowModule, modulesState))
	)
}
