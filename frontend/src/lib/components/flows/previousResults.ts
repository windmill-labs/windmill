import type { Schema } from '$lib/common'
import type { FlowModule, OpenFlow } from '$lib/gen'
import { schemaToObject } from '$lib/schema'
import { getAllSubmodules, getSubModules } from './flowExplorer'
import type { FlowState } from './flowState'

export type PickableProperties = {
	flow_input: Object
	priorIds: Record<string, any>
	previousId: string | undefined
	hasResume: boolean
}

type StepPropPicker = {
	pickableProperties: PickableProperties
	extraLib: string
}

type ModuleBranches = FlowModule[][]

export function dfs(id: string | undefined, flow: OpenFlow, getParents: boolean = true): FlowModule[] {
	if (id === undefined) {
		return []
	}

	function rec(id: string, moduleBranches: ModuleBranches): FlowModule[] | undefined {
		for (let modules of moduleBranches) {
			for (const [i, module] of modules.entries()) {
				if (module.id === id) {
					return getParents ? [module] : modules.slice(0, i + 1).reverse()
				} else {
					const submodules = getSubModules(module)

					if (submodules) {
						let found: FlowModule[] | undefined = rec(id, submodules)

						if (found) {
							return getParents ? [...found, module] : [...found, ...modules.slice(0, i).reverse()]
						}
					}
				}
			}
		}
		return undefined
	}

	return rec(id, [flow.value.modules]) ?? []
}

function getFlowInput(
	parentModules: FlowModule[],
	flowState: FlowState,
	args: any,
	schema: Schema
): Object {
	const parentModule = parentModules.shift()

	const topFlowInput = schemaToObject(schema, args)

	const parentState = parentModule ? flowState[parentModule.id] : undefined

	if (parentState && parentModule) {
		if (parentState.previewArgs) {
			return {...topFlowInput, ...parentState.previewArgs }
		} else {
			const parentFlowInput = getFlowInput(parentModules, flowState, args, schema)
			if (parentFlowInput.hasOwnProperty("iter")) {
				parentFlowInput["iter_parent"] = parentFlowInput["iter"]
				delete parentFlowInput["iter"]
			}
			if (topFlowInput.hasOwnProperty("iter")) {
				topFlowInput["iter_parent"] = topFlowInput["iter"]
				delete topFlowInput["iter"]
			}
			if (parentModule.value.type === 'forloopflow') {
				return {
					...topFlowInput,
					...parentFlowInput,
					iter: {
						value: "Iteration's value",
						index: "Iteration's index"
					},
				}
			} else {

				return {...topFlowInput,  ...parentFlowInput }
			}
		}
	} else {
		return topFlowInput
	}
}

export function getPreviousIds(id: string, flow: OpenFlow, include_node: boolean): string[] {
	const df = dfs(id, flow, false)
	if (!include_node) {
		df.shift()
	}
	return df
		.map((x) => {
			let submodules = getAllSubmodules(x)
				.flat()
				.map((x) => x.id)

			if (submodules.includes(id)) {
				return [x.id]
			} else {
				return [x.id, ...submodules]
			}
		})
		.flat()
}

export function getStepPropPicker(
	flowState: FlowState,
	parentModule: FlowModule | undefined,
	previousModule: FlowModule | undefined,
	id: string,
	flow: OpenFlow,
	args: any,
	include_node: boolean
): StepPropPicker {
	const flowInput = getFlowInput(
		dfs(parentModule?.id, flow),
		flowState,
		args,
		flow.schema as Schema
	)

	const previousIds = getPreviousIds(id, flow, include_node)

	let priorIds = Object.fromEntries(
		previousIds.map((id) => [id, flowState[id]?.previewResult ?? {}]).reverse()
	)

	const pickableProperties = {
		flow_input: flowInput,
		priorIds: priorIds,
		previousId: previousIds[0],
		hasResume: previousModule?.suspend != undefined
	}

	if (pickableProperties.hasResume) {
		pickableProperties['approvers'] = 'The list of approvers'
	}

	return {
		extraLib: buildExtraLib(flowInput, priorIds, previousModule?.suspend != undefined),
		pickableProperties
	}
}

export function buildExtraLib(
	flowInput: Record<string, any>,
	results: Record<string, any>,
	resume: boolean
): string {
	return `
/**
* get variable (including secret) at path
* @param {string} path - path of the variable (e.g: f/examples/secret)
*/
declare function variable(path: string): string;

/**
* get resource at path
* @param {string} path - path of the resource (e.g: f/examples/my_resource)
*/
declare function resource(path: string): any;

/**
* flow input as an object
*/
declare const flow_input = ${JSON.stringify(flowInput)};

/**
* static params of this same step
*/
declare const params: any;

/**
 * result by id
 */
declare const results = ${JSON.stringify(results)};

${
	resume
		? `
/**
 * resume payload
 */
declare const resume: any

/**
 * The list of approvers separated by ,
 */
declare const approvers: string
`
		: ''
}
`
}
