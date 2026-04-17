import { findModuleInFlow, findModuleParent } from '$lib/components/flows/flowDiff'
import type { FlowModule, OpenFlow } from '$lib/gen'

// Helper to find module by ID in a flow
export function getModuleById(flow: OpenFlow, moduleId: string): FlowModule | undefined {
	return findModuleInFlow(flow.value, moduleId) ?? undefined
}

export function getIndexInNestedModules(
	flow: OpenFlow,
	id: string
): { index: number; modules: FlowModule[] } | null {
	const parentLocation = findModuleParent(flow.value, id)
	if (!parentLocation) {
		// Module not found in flow.
		return null
	}

	if (parentLocation.type === 'failure' || parentLocation.type === 'preprocessor') {
		return null
	}

	if (parentLocation.type === 'root') {
		return {
			index: parentLocation.index,
			modules: flow.value.modules
		}
	}

	const parent = findModuleInFlow(flow.value, parentLocation.parentId)
	if (!parent) {
		return null
	}

	switch (parentLocation.type) {
		case 'forloop':
			if (parent.value.type !== 'forloopflow') {
				return null
			}
			return { index: parentLocation.index, modules: parent.value.modules }
		case 'whileloop':
			if (parent.value.type !== 'whileloopflow') {
				return null
			}
			return { index: parentLocation.index, modules: parent.value.modules }
		case 'branchone-default':
			if (parent.value.type !== 'branchone') {
				return null
			}
			return { index: parentLocation.index, modules: parent.value.default }
		case 'branchone-branch':
			if (parent.value.type !== 'branchone') {
				return null
			}
			return {
				index: parentLocation.index,
				modules: parent.value.branches[parentLocation.branchIndex]?.modules ?? []
			}
		case 'branchall-branch':
			if (parent.value.type !== 'branchall') {
				return null
			}
			return {
				index: parentLocation.index,
				modules: parent.value.branches[parentLocation.branchIndex]?.modules ?? []
			}
		case 'aiagent':
			if (parent.value.type !== 'aiagent') {
				return null
			}
			return {
				index: parentLocation.index,
				modules: (parent.value.tools as FlowModule[]) ?? []
			}
		default:
			return null
	}
}

/**
 * Collects all module IDs from an array of modules and their nested structures
 */
export function collectAllModuleIdsFromArray(modules: FlowModule[]): string[] {
	const ids: string[] = []
	for (const module of modules) {
		ids.push(...collectAllModuleIds(module))
	}
	return ids
}

/**
 * Recursively collects all module IDs from a module and its nested structures
 */
export function collectAllModuleIds(module: FlowModule): string[] {
	const ids: string[] = [module.id]

	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		if (module.value.modules) {
			for (const nested of module.value.modules) {
				ids.push(...collectAllModuleIds(nested))
			}
		}
	} else if (module.value.type === 'branchone') {
		if (module.value.branches) {
			for (const branch of module.value.branches) {
				if (branch.modules) {
					for (const nested of branch.modules) {
						ids.push(...collectAllModuleIds(nested))
					}
				}
			}
		}
		if (module.value.default) {
			for (const nested of module.value.default) {
				ids.push(...collectAllModuleIds(nested))
			}
		}
	} else if (module.value.type === 'branchall') {
		if (module.value.branches) {
			for (const branch of module.value.branches) {
				if (branch.modules) {
					for (const nested of branch.modules) {
						ids.push(...collectAllModuleIds(nested))
					}
				}
			}
		}
	} else if (module.value.type === 'aiagent') {
		if (module.value.tools) {
			for (const tool of module.value.tools) {
				ids.push(tool.id)
			}
		}
	}

	return ids
}
