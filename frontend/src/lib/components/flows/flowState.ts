import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { derived, writable } from 'svelte/store'
import { emptyFlowModuleSchema, isEmptyFlowModule, loadFlowModuleSchema } from './flowStateUtils'

export type FlowModuleState = {
	schema: Schema
	childFlowModules?: FlowModuleState[]
	previewResult?: any
}

export type FlowState = {
	modules: FlowModuleState[]
	failureModule: FlowModuleState
}

export const flowStateStore = writable<FlowState>(undefined)

export async function initFlowState(flow: Flow) {
	const modules = await mapFlowModules(flow.value.modules)

	const failureModule = flow.value.failure_module
		? await mapFlowModule(flow.value.failure_module)
		: emptyFlowModuleSchema()

	flowStateStore.set({
		modules,
		failureModule
	})
}



async function mapFlowModule(flowModule: FlowModule) {
	const value = flowModule.value
	if (value.type === 'forloopflow') {
		const childFlowModules = await Promise.all(
			value.modules.map(async (module) => loadFlowModuleSchema(module))
		)
		const loopFlowModule = await loadFlowModuleSchema(flowModule)

		return {
			...loopFlowModule,
			childFlowModules
		}
	}

	if (isEmptyFlowModule(flowModule)) {
		return emptyFlowModuleSchema()
	}

	return loadFlowModuleSchema(flowModule)
}

export async function mapFlowModules(flowModules: FlowModule[]): Promise<FlowModuleState[]> {
	return Promise.all(flowModules.map(async (flowModule: FlowModule) => mapFlowModule(flowModule)))
}
