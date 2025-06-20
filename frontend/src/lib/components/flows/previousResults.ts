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

export function dfs(
	id: string | undefined,
	flow: OpenFlow,
	getParents: boolean = true
): FlowModule[] {
	return dfsByModule(id, flow.value.modules, getParents)
}

export function dfsByModule(
	id: string | undefined,
	modules: FlowModule[],
	getParents: boolean = true
): FlowModule[] {
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

	return rec(id, [modules]) ?? []
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
		if (
			parentState.previewArgs &&
			parentState &&
			typeof parentState.previewArgs == 'object' &&
			`iter` in parentState.previewArgs
		) {
			return {
				iter: {
					value: "Iteration's value",
					index: "Iteration's index"
				},
				...topFlowInput,
				...parentState.previewArgs
			}
		} else {
			let parentFlowInput = getFlowInput(parentModules, flowState, args, schema)
			if (
				parentModule.value.type === 'forloopflow' ||
				parentModule.value.type === 'whileloopflow'
			) {
				let parentFlowInputIter = { ...parentFlowInput }
				if (parentFlowInputIter.hasOwnProperty('iter')) {
					parentFlowInputIter['iter_parent'] = parentFlowInputIter['iter']
					delete parentFlowInputIter['iter']
				}
				let topFlowInputIter = { ...topFlowInput }
				if (topFlowInputIter.hasOwnProperty('iter')) {
					topFlowInputIter['iter_parent'] = topFlowInputIter['iter']
					delete topFlowInputIter['iter']
				}
				return {
					...topFlowInputIter,
					...parentFlowInputIter,
					iter: {
						value: "Iteration's value",
						index: "Iteration's index"
					}
				}
			} else {
				return { ...topFlowInput, ...parentFlowInput }
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

export function getFailureStepPropPicker(flowState: FlowState, flow: OpenFlow, args: any) {
	const allIds = flow.value.modules.map((x) => x.id)
	let priorIds = Object.fromEntries(
		allIds.map((id) => [id, flowState[id]?.previewResult ?? {}]).reverse()
	)

	const flowInput = getFlowInput(
		dfs(flow.value.modules?.[0]?.id, flow),
		flowState,
		args,
		flow.schema as Schema
	)

	return {
		pickableProperties: {
			flow_input: schemaToObject(flow.schema as any, args),
			priorIds: priorIds,
			previousId: undefined,
			hasResume: false
		},
		extraLib: `
/**
* Error object
*/
declare const error: {
	message: string
	name: string
	stack: string
	step_id: string
}

/**
* result by id
*/
declare const results = ${JSON.stringify(priorIds)}

/**
* flow input as an object
*/
declare const flow_input = ${JSON.stringify(flowInput)};
		`
	}
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
		previousIds
			.map((id) => {
				const module = flow.value.modules.find((m) => m.id === id)
				return [
					id,
					module?.mock?.enabled
						? (module.mock.return_value ?? {})
						: (flowState[id]?.previewResult ?? {})
				]
			})
			.reverse()
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
		extraLib: buildExtraLib(
			flowInput,
			priorIds,
			previousModule?.suspend != undefined,
			previousModule?.id
		),
		pickableProperties
	}
}

export function buildExtraLib(
	flowInput: Record<string, any>,
	results: Record<string, any>,
	resume: boolean,
	previousId: string | undefined
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

/**
 * Result of the previous step
 */
declare const previous_result: ${previousId ? JSON.stringify(results[previousId]) : 'any'};

${resume
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

export function buildPrefixRegex(words: string[]): Array<{ regex: RegExp; word: string }> {
	return words.map((word) => {
		const prefixes: string[] = []
		for (let i = 1; i <= word.length; i++) {
			prefixes.push(word.slice(0, i) + '$')
		}
		prefixes.push(word + '\\.')
		prefixes.push(word + '\\[')

		return {
			regex: new RegExp(`^(${prefixes.join('|')}).*`),
			word
		}
	})
}

export function filterNestedObject(obj: any, nestedKeys: string[]) {
	if (nestedKeys.length === 0) return {}
	if (nestedKeys.length === 1) {
		if (nestedKeys[0] === '') {
			return obj
		}

		return Object.fromEntries(Object.entries(obj).filter(([key]) => key.startsWith(nestedKeys[0])))
	}
	const [key, ...rest] = nestedKeys
	if (obj && typeof obj === 'object' && key in obj) {
		const result = {}
		result[key] = filterNestedObject(obj[key], rest)
		return result
	}
	return {}
}

/**
 * Get the ID of the previous module within the same container (loop or branch)
 * based on the same logic used in FlowModuleWrapper.svelte
 */
export function getPreviousModule(moduleId: string, flow: OpenFlow): FlowModule | undefined {
	function searchInModules(modules: FlowModule[]): FlowModule | undefined | null {
		for (let i = 0; i < modules.length; i++) {
			const module = modules[i]

			if (module.id === moduleId) {
				// Found the module, return previous module ID if it exists
				return i > 0 ? modules[i - 1] : undefined
			}

			// Search in submodules based on module type
			if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
				const result = searchInModules(module.value.modules)
				if (result !== null) return result
			} else if (module.value.type === 'branchone') {
				// Search in default branch
				const defaultResult = searchInModules(module.value.default)
				if (defaultResult !== null) return defaultResult

				// Search in each branch
				for (const branch of module.value.branches) {
					const branchResult = searchInModules(branch.modules)
					if (branchResult !== null) return branchResult
				}
			} else if (module.value.type === 'branchall') {
				// Search in each branch
				for (const branch of module.value.branches) {
					const branchResult = searchInModules(branch.modules)
					if (branchResult !== null) return branchResult
				}
			}
		}
		return null // Continue searching (module not found in this branch)
	}

	const result = searchInModules(flow.value.modules)
	return result === null ? undefined : result
}
