import { dfs } from '$lib/components/flows/previousResults'
import type { OpenFlow } from '$lib/gen'

export function getIndexInNestedModules(flow: OpenFlow, id: string) {
	const accessingModules = dfs(id, flow, true).reverse()

	let parent = flow.value.modules
	let lastIndex = -1
	for (const [ai, am] of accessingModules.entries()) {
		const index = parent.findIndex((m) => m.id === am.id)

		if (index === -1) {
			throw new Error(`Module not found: ${am.id} in ${parent.map((m) => m.id).join(', ')}`)
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
				throw new Error(
					`Branch not found: ${am.id} in ${parent[index].value.branches.map((b) => b.modules.map((m) => m.id).join(', ')).join(';')}`
				)
			}
			parent = parent[index].value.branches[branchIdx].modules
		} else {
			throw new Error('Module is not a for loop or branch')
		}
	}

	if (lastIndex === -1) {
		throw new Error('Module not found, should have been caught earlier')
	}

	return {
		index: lastIndex,
		modules: parent
	}
}
export function getNestedModules(flow: OpenFlow, id: string, branchIndex?: number) {
	const { index, modules } = getIndexInNestedModules(flow, id)

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
	} else {
		throw new Error('Module is not a loop or branch')
	}
}
