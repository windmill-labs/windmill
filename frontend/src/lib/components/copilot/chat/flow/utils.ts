import { dfs } from '$lib/components/flows/previousResults'
import type { FlowModule, OpenFlow } from '$lib/gen'

// Helper to find module by ID in a flow
export function getModuleById(flow: OpenFlow, moduleId: string): FlowModule | undefined {
	const allModules = dfs(moduleId, flow, false)
	return allModules[0]
}

export function getIndexInNestedModules(flow: OpenFlow, id: string): { index: number; modules: FlowModule[] } | null {
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
export function getNestedModules(flow: OpenFlow, id: string, branchIndex?: number) {
	const result = getIndexInNestedModules(flow, id)
	if (!result) {
		throw new Error(`Module not found: ${id}`)
	}
	const { index, modules } = result

	// we know index is correct because we've already checked it in getIndexInNestedModules
	const module = modules[index]

	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		return module.value.modules
	} else if (
		branchIndex !== undefined &&
		(module.value.type === 'branchall' || module.value.type === 'branchone')
	) {
		if (module.value.type === 'branchone' && branchIndex === -1) {
			return module.value.default
		}

		const branch = module.value.branches[branchIndex]

		if (!branch) {
			throw new Error(
				`Branch not found: ${id} in ${module.value.branches.map((b) => b.modules.map((m) => m.id).join(', ')).join(';')}`
			)
		}

		return branch.modules
	} else if (module.value.type === 'aiagent') {
		return module.value.tools
	} else {
		throw new Error('Module is not a loop or branch')
	}
}
