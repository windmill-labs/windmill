import type { FlowModule, InputTransform, OpenFlow } from '$lib/gen'
import { isFlowModuleTool } from './agentToolUtils'

type ModuleBranches = FlowModule[][]

export function getSubModules(flowModule: FlowModule): ModuleBranches {
	if (flowModule.value.type === 'forloopflow' || flowModule.value.type === 'whileloopflow') {
		return [flowModule.value.modules]
	} else if (flowModule.value.type === 'branchall') {
		return flowModule.value.branches.map((branch) => branch.modules)
	} else if (flowModule.value.type == 'branchone') {
		return [...flowModule.value.branches.map((branch) => branch.modules), flowModule.value.default]
	} else if (flowModule.value.type === 'aiagent') {
		// Return AI agent tools as pseudo-FlowModules for searching
		if (flowModule.value.tools) {
			return [
				flowModule.value.tools
					.filter(isFlowModuleTool)
					.map(
						(tool) =>
							({
								id: tool.id,
								value: tool.value,
								summary: tool.summary
							}) as FlowModule
					)
			]
		}
	}
	return []
}

export function getAllSubmodules(flowModule: FlowModule): ModuleBranches {
	return getSubModules(flowModule).map((modules) => {
		return modules
			.map((module) => {
				return [module, ...getAllSubmodules(module).flat()]
			})
			.flat()
	})
}

export function getAllModules(
	flow_modules: FlowModule[],
	failure_module?: FlowModule
): FlowModule[] {
	let modules = [
		...flow_modules,
		...flow_modules.map((x) => getAllSubmodules(x).flat()),
		...(failure_module ? [failure_module] : [])
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

export function getDependentComponents(id: string, flow: OpenFlow): Record<string, string[]> {
	let modules = getAllModules(flow.value.modules, flow.value.failure_module)
	return filterDependentComponents(modules, id)
}

function filterDependentComponents(modules: FlowModule[], id: string): Record<string, string[]> {
	return id == 'Input'
		? Object.fromEntries(
				modules
					.map((mod) => [mod.id, getModuleExprs(mod).filter((expr) => expr.includes(`flow_input`))])
					.filter((x) => x[1].length > 0)
		  )
		: Object.fromEntries(
				modules
					.map((mod) => [
						mod.id,
						getModuleExprs(mod).filter((expr) => {
							const pattern = new RegExp(`\\bresults\\.${id}(?:\\b|\\.)`)
							return pattern.test(expr)
						})
					])
					.filter((x) => x[1].length > 0)
		  )
}

function getModuleExprs(x: FlowModule): string[] {
	let exprs: string[] = []
	if (x.value.type === 'forloopflow') {
		exprs.push(...getExpr(x.value.iterator))
	} else if (x.value.type === 'branchone') {
		x.value.branches.map((branch) => {
			exprs.push(branch.expr)
		})
	} else if (x.value.type === 'flow' || x.value.type === 'script' || x.value.type == 'rawscript') {
		exprs.push(...exprsOfInputTransforms(x.value.input_transforms))
		exprs.push(...getExpr(x.sleep))
		if (x.stop_after_if?.expr) {
			exprs.push(x.stop_after_if.expr)
		}
		if (x.stop_after_all_iters_if?.expr) {
			exprs.push(x.stop_after_all_iters_if.expr)
		}
		exprs.push(...getExpr(x.sleep))
	}
	return exprs
}

export function getDependeeAndDependentComponents(
	id: string,
	modules: FlowModule[],
	failure_module: FlowModule | undefined
): { dependees: Record<string, string[]>; dependents: Record<string, string[]> } {
	let all_modules = getAllModules(modules, failure_module)
	let module = all_modules.find((x) => x.id === id)
	let allIds: [string, string][] = [
		['Input', 'flow_input'],
		...all_modules.map((x) => [x.id, `results.${x.id}`] as [string, string])
	]
	let dependees = {}
	if (module) {
		getModuleExprs(module).forEach((expr) => {
			allIds.forEach((y) => {
				if (expr.includes(y[1])) {
					dependees[y[0]] = dependees[y[0]] ?? []
					dependees[y[0]].push(expr)
				}
			})
		})
	}
	let dependents = filterDependentComponents(all_modules, id)
	return { dependees, dependents }
}

// export function getAllDependencies(
// 	flow_modules: FlowModule[],
// 	failure_module: FlowModule | undefined
// ): Record<string, string[]> {
// 	let modules = getAllModules(flow_modules, failure_module)
// 	let allIds: [string, string][] = [
// 		['flow_input', 'flow_input'],
// 		...modules.map((x) => [x.id, `results.${x.id}`] as [string, string])
// 	]
// 	let deps: Record<string, string[]> = {}
// 	function filterExprs(source, ...exprs: string[]) {
// 		exprs.forEach((x) => {
// 			let f = allIds.find((y) => x.includes(y[1]))
// 			if (f) {
// 				deps[source] = deps[source] ?? []
// 				deps[source].push(f[0])
// 			}
// 		})
// 	}

// 	modules.forEach((x) => {
// 		if (x.value.type === 'forloopflow') {
// 			filterExprs(x.id, ...getExpr(x.value.iterator))
// 		} else if (x.value.type === 'branchone') {
// 			x.value.branches.map((branch) => {
// 				filterExprs(x.id, branch.expr)
// 			})
// 		} else if (
// 			x.value.type === 'flow' ||
// 			x.value.type === 'script' ||
// 			x.value.type == 'rawscript'
// 		) {
// 			filterExprs(x.id, ...exprsOfInputTransforms(x.value.input_transforms))
// 			filterExprs(x.id, ...getExpr(x.sleep))
// 			if (x.stop_after_if?.expr) {
// 				filterExprs(x.id, x.stop_after_if.expr)
// 			}
// 			filterExprs(x.id, ...getExpr(x.sleep))
// 		}
// 	})
// 	return deps
// }
