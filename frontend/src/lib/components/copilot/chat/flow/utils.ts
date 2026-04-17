import {
	findModuleInFlow,
	getModuleArrayContainer
} from '$lib/components/flows/flowTree'
import type { FlowModule, OpenFlow } from '$lib/gen'

// Helper to find module by ID in a flow
export function getModuleById(flow: OpenFlow, moduleId: string): FlowModule | undefined {
	return findModuleInFlow(flow.value, moduleId) ?? undefined
}

export function getIndexInNestedModules(
	flow: OpenFlow,
	id: string
): { index: number; modules: FlowModule[] } | null {
	return getModuleArrayContainer(flow.value, id)
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
