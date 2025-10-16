import type { FlowModule } from '$lib/gen'

export function dfs<T>(
	modules: FlowModule[],
	f: (x: FlowModule, modules: FlowModule[], branches: FlowModule[][]) => T,
	opts: { skipToolNodes?: boolean } = {}
): T[] {
	let result: T[] = []
	for (const module of modules) {
		if (module.value.type == 'forloopflow' || module.value.type == 'whileloopflow') {
			result = result.concat(f(module, modules, [module.value.modules]))
			result = result.concat(dfs(module.value.modules, f, opts))
		} else if (module.value.type == 'branchone') {
			const allBranches = [module.value.default, ...module.value.branches.map((b) => b.modules)]
			result = result.concat(f(module, modules, allBranches))

			for (const branch of allBranches) {
				result = result.concat(dfs(branch, f, opts))
			}
		} else if (module.value.type == 'branchall') {
			const allBranches = module.value.branches.map((b) => b.modules)
			result = result.concat(f(module, modules, allBranches))
			for (const branch of allBranches) {
				result = result.concat(dfs(branch, f, opts))
			}
		} else if (module.value.type == 'aiagent' && !opts.skipToolNodes) {
			// // Convert AgentTool[] to FlowModule[] for traversal
			// // Only FlowModule tools can be traversed (MCP tools are leaf nodes)
			// const toolsAsModules = module.value.tools
			// 	.map(agentToolToFlowModule)
			// 	.filter((m): m is FlowModule => m !== undefined)
			// result = result.concat(f(module, modules, [toolsAsModules]))
			// result = result.concat(dfs(toolsAsModules, f, opts))
			result.push(f(module, modules, []))
		} else {
			result.push(f(module, modules, []))
		}
	}
	return result
}
