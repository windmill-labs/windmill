import type { Schema } from '$lib/common'
import type { Flow, FlowModule, Job } from '$lib/gen'
import { buildExtraLib, objectToTsType, schemaToObject } from '$lib/utils'
import type { FlowState } from './flowState'

type Result = any

type PickableProperties = {
	flow_input?: Object
	previous_result: Result | undefined
	step?: Result[]
}

type StepPropPicker = {
	pickableProperties: PickableProperties
	extraLib: string
}

type ParentModule = {
	parentModule: FlowModule
	parentPreviousModuleId: string | undefined
}
type ModuleBranches = FlowModule[][]

function dfs(id: string | undefined, flow: Flow): ParentModule[] {
	if (id === undefined) {
		return []
	}
	function getSubModules(flowModule: FlowModule): ModuleBranches {
		if (flowModule.value.type === 'forloopflow') {
			return [flowModule.value.modules]
		} else if (flowModule.value.type === 'branchall') {
			return flowModule.value.branches.map((branch) => branch.modules)
		} else if (flowModule.value.type == 'branchone') {
			return [
				...flowModule.value.branches.map((branch) => branch.modules),
				flowModule.value.default
			]
		}
		return []
	}

	function rec(id: string, moduleBranches: ModuleBranches): ParentModule[] | undefined {
		for (let modules of moduleBranches) {
			let parentPreviousModuleId: string | undefined = undefined

			for (let module of modules) {
				if (module.id === id) {
					return [{ parentModule: module, parentPreviousModuleId }]
				} else {
					const submodules = getSubModules(module)

					if (submodules) {
						let found: ParentModule[] | undefined = rec(id, submodules)

						if (module && found) {
							return [...found, { parentModule: module, parentPreviousModuleId }]
						}
					}
				}
				parentPreviousModuleId = module.id
			}
		}
		return undefined
	}

	return rec(id, [flow.value.modules]) ?? []
}

function flattenPreviousResult(pr: any) {
	if (typeof pr === 'object' && pr.previous_result) {
		return pr.previous_result
	}

	return pr
}

function getFlowInput(
	parentModules: ParentModule[],
	flowState: FlowState,
	args: any,
	schema: Schema
) {
	const { parentModule, parentPreviousModuleId } = parentModules.shift() ?? {
		parentModule: undefined,
		parentPreviousModuleId: undefined
	}

	const parentState = parentModule ? flowState[parentModule.id] : undefined

	if (parentState && parentModule) {
		if (parentState.previewArgs) {
			return parentState.previewArgs
		} else {
			const parentFlowInput = getFlowInput(parentModules, flowState, args, schema)

			if (parentModule.value.type === 'forloopflow') {
				return {
					...parentFlowInput,
					iter: {
						value: "Iteration's value",
						index: "Iteration's index"
					}
				}
			} else {
				// Branches

				if (parentPreviousModuleId === undefined) {
					return parentFlowInput
				} else {
					return {
						...parentFlowInput,
						previous_result: flattenPreviousResult(flowState[parentPreviousModuleId].previewResult)
					}
				}
			}
		}
	} else {
		return schemaToObject(schema, args)
	}
}

function getPriorIds(flow: Flow, id: string): string[] {
	// TODO: Ruben
	return flow.value.modules.map((module) => module.id)
}

export function getStepPropPicker(
	flowState: FlowState,
	parentModule: FlowModule | undefined,
	previousModuleId: string | undefined,
	flow: Flow,
	args: any
): StepPropPicker {
	const flowInput = getFlowInput(dfs(parentModule?.id, flow), flowState, args, flow.schema)

	const previousResults = previousModuleId
		? flowState[previousModuleId].previewResult
		: flattenPreviousResult(flowInput)
	// const priorIds = getPriorIds(flow, parentModule.id)

	return {
		extraLib: buildExtraLib(objectToTsType(flowInput), objectToTsType(previousResults)),
		pickableProperties: {
			flow_input: flowInput,
			previous_result: previousResults
		}
	}
}
