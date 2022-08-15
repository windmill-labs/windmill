import type { Schema } from '$lib/common'
import type { Flow, FlowModule } from '$lib/gen'
import { writable } from 'svelte/store'
import { emptyFlowModuleSchema, loadFlowModuleSchema } from './flowStateUtils'

export type FlowModuleSchema = {
	flowModule: FlowModule
	schema: Schema
	childFlowModules?: FlowModuleSchema[]
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
	flow.value.modules = flow.value.modules.map((module, index) => {
		const correspondingModule = flowState[index].flowModule
		if (correspondingModule.value.type === 'forloopflow') {
			correspondingModule.value.value.modules = flowState[index].childFlowModules!.map(
				(cfm) => cfm.flowModule
			)
			module = correspondingModule
		} else {
			module = flowState[index].flowModule
		}

		return module
	})
	return flow
}

export function addStep(stepIndexes: number[]) {
	const isInsideLoop = stepIndexes.length > 1

	flowStateStore.update((flowState: FlowState) => {
		const [parentStepIndex, childStepIndex] = stepIndexes

		const parentStep = flowState[parentStepIndex]

		if (isInsideLoop && Array.isArray(parentStep.childFlowModules)) {
			parentStep.childFlowModules.splice(childStepIndex, 0, emptyFlowModuleSchema())

			flowState.splice(parentStepIndex, 1, {
				...parentStep,
				childFlowModules: parentStep.childFlowModules
			})
		} else {
			flowState.splice(parentStepIndex, 0, emptyFlowModuleSchema())
		}

		return flowState
	})
}

export function removeStep(stepIndexes: number[]) {
	const isInsideLoop = stepIndexes.length > 1

	flowStateStore.update((flowState: FlowState) => {
		const [parentStepIndex, childStepIndex] = stepIndexes

		const parentStep = flowState[parentStepIndex]

		if (
			isInsideLoop &&
			Array.isArray(parentStep.childFlowModules) &&
			// When we remove the last element of the loop
			// We can remove the loop
			parentStep.childFlowModules.length > 1
		) {
			parentStep.childFlowModules.splice(childStepIndex, 1)
			flowState.splice(parentStepIndex, 0, {
				...parentStep,
				childFlowModules: parentStep.childFlowModules
			})
		} else {
			flowState.splice(parentStepIndex, 1)
		}

		return flowState
	})
}
