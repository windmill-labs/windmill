import type { FlowModule } from '$lib/gen'

type FlowDfsOptions = { skipToolNodes?: boolean }

type FlowModuleVisitor<T> = (
	x: FlowModule,
	modules: FlowModule[],
	branches: FlowModule[][]
) => T

function traverseFlowModules(
	modules: FlowModule[],
	visit: FlowModuleVisitor<void>,
	opts: FlowDfsOptions = {}
): void {
	for (const module of modules) {
		if (module.value.type == 'forloopflow' || module.value.type == 'whileloopflow') {
			visit(module, modules, [module.value.modules])
			traverseFlowModules(module.value.modules, visit, opts)
		} else if (module.value.type == 'branchone') {
			const allBranches = [module.value.default, ...module.value.branches.map((b) => b.modules)]
			visit(module, modules, allBranches)

			for (const branch of allBranches) {
				traverseFlowModules(branch, visit, opts)
			}
		} else if (module.value.type == 'branchall') {
			const allBranches = module.value.branches.map((b) => b.modules)
			visit(module, modules, allBranches)
			for (const branch of allBranches) {
				traverseFlowModules(branch, visit, opts)
			}
		} else if (module.value.type == 'aiagent' && !opts.skipToolNodes) {
			visit(module, modules, [module.value.tools as FlowModule[]])
			traverseFlowModules(module.value.tools as FlowModule[], visit, opts)
		} else {
			visit(module, modules, [])
		}
	}
}

export function dfs<T>(
	modules: FlowModule[],
	f: FlowModuleVisitor<T>,
	opts: FlowDfsOptions = {}
): T[] {
	let result: T[] = []
	traverseFlowModules(modules, (module, parentModules, branches) => {
		result.push(f(module, parentModules, branches))
	}, opts)
	return result
}

export function forEachFlowModule(
	modules: FlowModule[],
	f: FlowModuleVisitor<void>,
	opts: FlowDfsOptions = {}
): void {
	traverseFlowModules(modules, f, opts)
}
