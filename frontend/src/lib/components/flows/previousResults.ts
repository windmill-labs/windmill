import type { Schema } from '$lib/common'
import type { Flow, FlowModule, InputTransform } from '$lib/gen'
import { schemaToObject } from '$lib/schema'
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

function getSubModules(flowModule: FlowModule): ModuleBranches {
	if (flowModule.value.type === 'forloopflow') {
		return [flowModule.value.modules]
	} else if (flowModule.value.type === 'branchall') {
		return flowModule.value.branches.map((branch) => branch.modules)
	} else if (flowModule.value.type == 'branchone') {
		return [...flowModule.value.branches.map((branch) => branch.modules), flowModule.value.default]
	}
	return []
}

function getAllSubmodules(flowModule: FlowModule): ModuleBranches {
	return getSubModules(flowModule).map((modules) => {
		return modules
			.map((module) => {
				return [module, ...getAllSubmodules(module).flat()]
			})
			.flat()
	})
}

function dfs(id: string | undefined, flow: Flow, getParents: boolean = true): FlowModule[] {
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
) {
	const parentModule = parentModules.shift()

	const parentState = parentModule ? flowState[parentModule.id] : undefined

	if (parentState && parentModule) {
		if (parentState.previewArgs) {
			return parentState.previewArgs
		} else {
			const parentFlowInput = getFlowInput(parentModules, flowState, args, schema)

			if (parentModule.value.type === 'forloopflow') {
				return {
					iter: {
						value: "Iteration's value",
						index: "Iteration's index"
					},
					...parentFlowInput
				}
			} else {
				return parentFlowInput
			}
		}
	} else {
		return schemaToObject(schema, args)
	}
}

export function getAllModules(flow: Flow): FlowModule[] {
	let modules = [
		...flow.value.modules,
		...flow.value.modules.map((x) => getAllSubmodules(x).flat()),
		...(flow.value.failure_module ? [flow.value.failure_module] : [])
	].flat()
	return modules
}
function getExpr(x: InputTransform | undefined) {
	if (x == undefined) return []
	return x.type === 'javascript' ? [x.expr] : []
}

function exprsOfInputTransforms(x: Record<string, InputTransform>): string[] {
	return Object.values(x)
		.map((x) => getExpr(x))
		.flat()
}
export function getDependentComponents(id: string, flow: Flow): Record<string, string[]> {
	let modules = getAllModules(flow)
	return Object.fromEntries(
		modules
			.map((x) => {
				let exprs: string[] = []
				if (x.value.type === 'forloopflow') {
					exprs.push(...getExpr(x.value.iterator))
				} else if (x.value.type === 'branchone') {
					x.value.branches.map((branch) => {
						exprs.push(branch.expr)
					})
				} else if (
					x.value.type === 'flow' ||
					x.value.type === 'script' ||
					x.value.type == 'rawscript'
				) {
					exprs.push(...exprsOfInputTransforms(x.value.input_transforms))
					exprs.push(...getExpr(x.sleep))
					if (x.stop_after_if?.expr) {
						exprs.push(x.stop_after_if.expr)
					}
					exprs.push(...getExpr(x.sleep))
				}
				exprs = exprs.filter((x) => x.includes(`results.${id}`))
				return [x.id, exprs]
			})
			.filter((x) => x[1].length > 0)
	)
}
export function getStepPropPicker(
	flowState: FlowState,
	parentModule: FlowModule | undefined,
	previousModule: FlowModule | undefined,
	id: string,
	flow: Flow,
	args: any,
	include_node: boolean
): StepPropPicker {
	const flowInput = getFlowInput(
		dfs(parentModule?.id, flow),
		flowState,
		args,
		flow.schema as Schema
	)

	const previousIds = dfs(id, flow, false)
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
	if (!include_node) {
		previousIds.shift()
	}

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
