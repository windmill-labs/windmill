import type { FlowModule, FlowValue } from '$lib/gen'

export type ModuleParentLocation =
	| { type: 'root'; index: number }
	| { type: 'forloop' | 'whileloop'; parentId: string; index: number }
	| { type: 'branchone-default'; parentId: string; index: number }
	| { type: 'branchone-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'branchall-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'aiagent'; parentId: string; index: number }
	| { type: 'failure'; index: -1 }
	| { type: 'preprocessor'; index: -1 }

type ArrayBackedModuleParentLocation = Exclude<
	ModuleParentLocation,
	{ type: 'failure'; index: -1 } | { type: 'preprocessor'; index: -1 }
>

export type FlowNodeMatch =
	| {
			module: FlowModule
			location: ArrayBackedModuleParentLocation
			container: FlowModule[]
	  }
	| {
			module: FlowModule
			location: Extract<ModuleParentLocation, { type: 'failure' | 'preprocessor' }>
	  }

export function findFlowNode(flow: FlowValue, moduleId: string): FlowNodeMatch | null {
	if (flow.failure_module?.id === moduleId) {
		return {
			module: flow.failure_module,
			location: { type: 'failure', index: -1 }
		}
	}

	if (flow.preprocessor_module?.id === moduleId) {
		return {
			module: flow.preprocessor_module,
			location: { type: 'preprocessor', index: -1 }
		}
	}

	function searchInModules(
		modules: FlowModule[],
		locationBuilder: (index: number) => ArrayBackedModuleParentLocation
	): FlowNodeMatch | null {
		const matchIndex = modules.findIndex((module) => module.id === moduleId)
		if (matchIndex >= 0) {
			return {
				module: modules[matchIndex],
				location: locationBuilder(matchIndex),
				container: modules
			}
		}

		for (const module of modules) {
			if (module.value.type === 'forloopflow') {
				const nested = searchInModules(module.value.modules, (index) => ({
					type: 'forloop',
					parentId: module.id,
					index
				}))
				if (nested) return nested
			}

			if (module.value.type === 'whileloopflow') {
				const nested = searchInModules(module.value.modules, (index) => ({
					type: 'whileloop',
					parentId: module.id,
					index
				}))
				if (nested) return nested
			}

			if (module.value.type === 'branchone') {
				const defaultNested = searchInModules(module.value.default, (index) => ({
					type: 'branchone-default',
					parentId: module.id,
					index
				}))
				if (defaultNested) return defaultNested

				for (let branchIndex = 0; branchIndex < module.value.branches.length; branchIndex++) {
					const branch = module.value.branches[branchIndex]
					const nested = searchInModules(branch.modules, (index) => ({
						type: 'branchone-branch',
						parentId: module.id,
						branchIndex,
						index
					}))
					if (nested) return nested
				}
			}

			if (module.value.type === 'branchall') {
				for (let branchIndex = 0; branchIndex < module.value.branches.length; branchIndex++) {
					const branch = module.value.branches[branchIndex]
					const nested = searchInModules(branch.modules, (index) => ({
						type: 'branchall-branch',
						parentId: module.id,
						branchIndex,
						index
					}))
					if (nested) return nested
				}
			}

			if (module.value.type === 'aiagent' && module.value.tools) {
				const nested = searchInModules(module.value.tools as FlowModule[], (index) => ({
					type: 'aiagent',
					parentId: module.id,
					index
				}))
				if (nested) return nested
			}
		}

		return null
	}

	return searchInModules(flow.modules ?? [], (index) => ({ type: 'root', index }))
}

export function findModuleInFlow(flow: FlowValue, moduleId: string): FlowModule | null {
	return findFlowNode(flow, moduleId)?.module ?? null
}

export function findModuleParent(flow: FlowValue, moduleId: string): ModuleParentLocation | null {
	return findFlowNode(flow, moduleId)?.location ?? null
}

export function getModuleArrayContainer(
	flow: FlowValue,
	moduleId: string
): { index: number; modules: FlowModule[] } | null {
	const match = findFlowNode(flow, moduleId)
	if (!match || !('container' in match)) {
		return null
	}

	return {
		index: match.location.index,
		modules: match.container
	}
}
