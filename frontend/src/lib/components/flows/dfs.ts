import type { FlowModule } from '$lib/gen'

export function dfs<T>(modules: FlowModule[], f: (x: FlowModule) => T): T[] {
	let result: T[] = []
	for (const module of modules) {
		if (module.value.type == 'forloopflow' || module.value.type == 'whileloopflow') {
			result = result.concat(f(module))
			result = result.concat(dfs(module.value.modules, f))
		} else if (module.value.type == 'branchone') {
			result = result.concat(f(module))
			result = result.concat(
				dfs(
					module.value.branches
						.map((b) => b.modules)
						.flat()
						.concat(module.value.default),
					f
				)
			)
		} else if (module.value.type == 'branchall') {
			result = result.concat(f(module))
			result = result.concat(dfs(module.value.branches.map((b) => b.modules).flat(), f))
		} else {
			result.push(f(module))
		}
	}
	return result
}

export function isParent(module: FlowModule, targetId: string): FlowModule | undefined {
	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		if (module.value.modules?.some((child) => child.id === targetId)) {
			return module
		}
	} else if (module.value.type === 'branchone') {
		if (
			module.value.branches?.some((branch) =>
				branch.modules.some((child) => child.id === targetId)
			) ||
			module.value.default?.some((child) => child.id === targetId)
		) {
			return module
		}
	} else if (module.value.type === 'branchall') {
		if (
			module.value.branches?.some((branch) => branch.modules.some((child) => child.id === targetId))
		) {
			return module
		}
	}
	return undefined
}
