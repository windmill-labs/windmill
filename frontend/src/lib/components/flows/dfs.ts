import type { FlowModule } from '$lib/gen/models/FlowModule'

export function dfs<T>(modules: FlowModule[], f: (x: FlowModule) => T): T[] {
	let result: T[] = []
	for (const module of modules) {
		if (module.value.type == 'forloopflow') {
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
