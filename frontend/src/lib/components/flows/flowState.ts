import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { writable } from 'svelte/store'
import { loadFlowModuleSchema } from './flowStateUtils'

export type FlowModuleSchema = {
	flowModule: FlowModule
	schema: Schema
	childFlowModules?: FlowModuleSchema[]
	previewResults: Array<any>
}

export type FlowState = FlowModuleSchema[]

export const flowStateStore = writable<FlowState>(undefined)

export async function initFlowState(flow: Flow) {
	const flowState = await flowModulesToFlowState(flow.value.modules)
	flowStateStore.set(flowState)
}

export async function flowModulesToFlowState(flowModules: FlowModule[]): Promise<FlowState> {
	return Promise.all(
		flowModules.map(async (flowModule: FlowModule) => {
			const value = flowModule.value
			if (value.type === 'forloopflow') {
				const childFlowModules = await Promise.all(
					value.value.modules.map(async (module) => loadFlowModuleSchema(module))
				)
				const loopFlowModule = await loadFlowModuleSchema(flowModule)

				return {
					...loopFlowModule,
					childFlowModules
				}
			}
			return loadFlowModuleSchema(flowModule)
		})
	)
}

export function flowStateToFlow(flowState: FlowState, flow: Flow): Flow {
	if (!flowState) {
		return flow
	}

	const modules = flowState.map(({ flowModule, childFlowModules }, index) => {
		const fmv = flowModule.value

		if (fmv.type === 'forloopflow') {
			fmv.value.modules = childFlowModules!.map((cfm) => cfm.flowModule)
			flowModule.value = fmv
		}

		return flowModule
	})

	flow.value.modules = modules
	return flow
}
