import { dfs } from '$lib/components/flows/previousResults'
import type { FlowModule, OpenFlow } from '$lib/gen'

// Helper to find module by ID in a flow
export function getModuleById(flow: OpenFlow, moduleId: string): FlowModule | undefined {
	const allModules = dfs(moduleId, flow, false)
	return allModules[0]
}

export function getIndexInNestedModules(
	flow: OpenFlow,
	id: string
): { index: number; modules: FlowModule[] } | null {
	const accessingModules = dfs(id, flow, true).reverse()

	if (accessingModules.length === 0) {
		// Module not found in flow
		return null
	}

	let parent = flow.value.modules
	let lastIndex = -1
	for (const [ai, am] of accessingModules.entries()) {
		const index = parent.findIndex((m) => m.id === am.id)

		if (index === -1) {
			// Module no longer exists in expected location (may have been deleted with parent)
			return null
		}

		lastIndex = index

		if (ai === accessingModules.length - 1) {
			break
		}

		if (
			parent[index].value.type === 'forloopflow' ||
			parent[index].value.type === 'whileloopflow'
		) {
			parent = parent[index].value.modules
		} else if (
			parent[index].value.type === 'branchall' ||
			parent[index].value.type === 'branchone'
		) {
			const branchIdx = parent[index].value.branches.findIndex((b) =>
				b.modules.some((m) => m.id === accessingModules[ai + 1].id)
			)
			if (branchIdx === -1) {
				// Module no longer exists in branch (may have been deleted)
				return null
			}
			parent = parent[index].value.branches[branchIdx].modules
		} else {
			// Unexpected module type in path
			return null
		}
	}

	if (lastIndex === -1) {
		return null
	}

	return {
		index: lastIndex,
		modules: parent
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
