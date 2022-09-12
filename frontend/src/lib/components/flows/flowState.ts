import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { derived, writable } from 'svelte/store'
import { emptyFlowModuleSchema, isEmptyFlowModule, loadFlowModuleSchema } from './flowStateUtils'

export type FlowModuleSchema = {
	flowModule: FlowModule
	schema: Schema
	childFlowModules?: FlowModuleSchema[]
	previewResult?: any
}

export type FlowState = {
	modules: FlowModuleSchema[]
	failureModule: FlowModuleSchema
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

export const isCopyFirstStepSchemaDisabled = derived(
	flowStateStore,
	(flowState: FlowState | undefined) => {
		if (flowState) {
			const firstModule = flowState.modules[0]
			if (!firstModule) {
				return true
			}
			const fm = firstModule.flowModule
			return flowState.modules.length === 0 || isEmptyFlowModule(fm)
		} else {
			return true
		}
	}
)

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

export async function mapFlowModules(flowModules: FlowModule[]): Promise<FlowModuleSchema[]> {
	return Promise.all(flowModules.map(async (flowModule: FlowModule) => mapFlowModule(flowModule)))
}

export function flowStateToFlow(flowState: FlowState, flow: Flow): Flow {
	if (!flowState || !flow) {
		return flow
	}

	const modules = flowState.modules.map(({ flowModule, childFlowModules }) => {
		const fmv = flowModule.value

		if (fmv.type === 'forloopflow' && childFlowModules && Array.isArray(childFlowModules)) {
			fmv.modules = childFlowModules.map((cfm) => cfm.flowModule)
			flowModule.value = fmv
		}

		return flowModule
	})

	flow.value.modules = modules
	return flow
}
