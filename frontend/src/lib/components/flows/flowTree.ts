import type { FlowModule } from '$lib/gen'

export type FlowModuleTree = {
	modules?: FlowModule[] | null
	failure_module?: FlowModule | null
	preprocessor_module?: FlowModule | null
}

export type ModuleParentLocation =
	| { type: 'root'; index: number }
	| { type: 'forloop' | 'whileloop'; parentId: string; index: number }
	| { type: 'branchone-default'; parentId: string; index: number }
	| { type: 'branchone-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'branchall-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'aiagent'; parentId: string; index: number }
	| { type: 'failure'; index: -1 }
	| { type: 'preprocessor'; index: -1 }

export type ArrayBackedModuleParentLocation = Exclude<
	ModuleParentLocation,
	{ type: 'failure'; index: -1 } | { type: 'preprocessor'; index: -1 }
>

type ArrayBackedFlowNodeMatch = {
	module: FlowModule
	location: ArrayBackedModuleParentLocation
	container: FlowModule[]
}

export type FlowNodeMatch =
	| ArrayBackedFlowNodeMatch
	| {
			module: FlowModule
			location: Extract<ModuleParentLocation, { type: 'failure' | 'preprocessor' }>
	  }

export type FlowNodeLocation = {
	module: FlowModule
	location: ModuleParentLocation
}

function isFlowModuleToolCandidate(tool: FlowModule): boolean {
	const toolValue = tool.value as FlowModule['value'] & { tool_type?: string }
	return toolValue.tool_type === undefined || toolValue.tool_type === 'flowmodule'
}

export function getChildModuleBranches(module: FlowModule): FlowModule[][] {
	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		return [module.value.modules]
	}

	if (module.value.type === 'branchone') {
		return [module.value.default, ...module.value.branches.map((branch) => branch.modules)]
	}

	if (module.value.type === 'branchall') {
		return module.value.branches.map((branch) => branch.modules)
	}

	if (module.value.type === 'aiagent' && module.value.tools) {
		return [
			(module.value.tools as FlowModule[]).filter((tool) => isFlowModuleToolCandidate(tool))
		]
	}

	return []
}

export function collectDescendantFlowModules(module: FlowModule): FlowModule[] {
	return getChildModuleBranches(module).flatMap((branch) =>
		branch.flatMap((childModule) => [childModule, ...collectDescendantFlowModules(childModule)])
	)
}

function visitNestedFlowNodesOfModule(
	module: FlowModule,
	visit: (match: ArrayBackedFlowNodeMatch) => boolean | void
): boolean {
	if (module.value.type === 'aiagent' && module.value.tools) {
		const tools = module.value.tools as FlowModule[]
		for (let toolIndex = 0; toolIndex < tools.length; toolIndex++) {
			const tool = tools[toolIndex]
			if (!isFlowModuleToolCandidate(tool as FlowModule)) {
				continue
			}

			const toolMatch: ArrayBackedFlowNodeMatch = {
				module: tool as FlowModule,
				location: {
					type: 'aiagent',
					parentId: module.id,
					index: toolIndex
				},
				container: tools as FlowModule[]
			}

			if (visit(toolMatch) || visitNestedFlowNodesOfModule(tool as FlowModule, visit)) {
				return true
			}
		}

		return false
	}

	const childBranches = getChildModuleBranches(module)
	for (let branchIndex = 0; branchIndex < childBranches.length; branchIndex++) {
		const branch = childBranches[branchIndex]
		const locationBuilder =
			module.value.type === 'forloopflow'
				? (childIndex: number) =>
						({
							type: 'forloop',
							parentId: module.id,
							index: childIndex
						}) as ArrayBackedModuleParentLocation
				: module.value.type === 'whileloopflow'
					? (childIndex: number) =>
							({
								type: 'whileloop',
								parentId: module.id,
								index: childIndex
							}) as ArrayBackedModuleParentLocation
					: module.value.type === 'branchone'
						? branchIndex === 0
							? (childIndex: number) =>
									({
										type: 'branchone-default',
										parentId: module.id,
										index: childIndex
									}) as ArrayBackedModuleParentLocation
							: (childIndex: number) =>
									({
										type: 'branchone-branch',
										parentId: module.id,
										branchIndex: branchIndex - 1,
										index: childIndex
									}) as ArrayBackedModuleParentLocation
						: (childIndex: number) =>
								({
									type: 'branchall-branch',
									parentId: module.id,
									branchIndex,
									index: childIndex
								}) as ArrayBackedModuleParentLocation

		if (visitFlowNodesInModules(branch, locationBuilder, visit)) {
			return true
		}
	}

	return false
}

function visitFlowNodesInModules(
	modules: FlowModule[],
	locationBuilder: (index: number) => ArrayBackedModuleParentLocation,
	visit: (match: ArrayBackedFlowNodeMatch) => boolean | void
): boolean {
	for (let index = 0; index < modules.length; index++) {
		const module = modules[index]

		if (
			visit({
				module,
				location: locationBuilder(index),
				container: modules
			})
		) {
			return true
		}

		if (visitNestedFlowNodesOfModule(module, visit)) {
			return true
		}
	}

	return false
}

function findFlowNodeInModules(
	modules: FlowModule[],
	moduleId: string,
	locationBuilder: (index: number) => ArrayBackedModuleParentLocation
): ArrayBackedFlowNodeMatch | null {
	let match: ArrayBackedFlowNodeMatch | null = null

	visitFlowNodesInModules(modules, locationBuilder, (nodeMatch) => {
		if (nodeMatch.module.id !== moduleId) {
			return false
		}

		match = nodeMatch
		return true
	})

	return match
}

/**
 * Collect every module ID in a flow tree, including ALL aiagent tool IDs
 * (both flowmodule and non-flowmodule like MCP).  Used for ID uniqueness
 * validation where every tool's ID must be reserved regardless of type.
 */
export function collectAllFlowModuleIds(flow: FlowModuleTree): string[] {
	const ids: string[] = []

	function walkModules(modules: FlowModule[]): void {
		for (const module of modules) {
			if (module.id) {
				ids.push(module.id)
			}

			if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
				walkModules(module.value.modules)
			} else if (module.value.type === 'branchone') {
				for (const branch of module.value.branches) {
					walkModules(branch.modules)
				}
				walkModules(module.value.default)
			} else if (module.value.type === 'branchall') {
				for (const branch of module.value.branches) {
					walkModules(branch.modules)
				}
			} else if (module.value.type === 'aiagent' && module.value.tools) {
				walkModules(module.value.tools as FlowModule[])
			}
		}
	}

	if (flow.modules) {
		walkModules(flow.modules)
	}
	if (flow.failure_module?.id) {
		ids.push(flow.failure_module.id)
	}
	if (flow.preprocessor_module?.id) {
		ids.push(flow.preprocessor_module.id)
	}

	return ids
}

/**
 * Like collectAllFlowModuleIds but operates on a modules array directly
 * (without failure_module/preprocessor_module).
 */
export function collectAllFlowModuleIdsFromModules(modules: FlowModule[]): string[] {
	return collectAllFlowModuleIds({ modules })
}

export function collectFlowNodes(flow: FlowModuleTree): FlowNodeLocation[] {
	const matches: FlowNodeLocation[] = []

	if (flow.failure_module) {
		matches.push({
			module: flow.failure_module,
			location: { type: 'failure', index: -1 }
		})
	}

	if (flow.preprocessor_module) {
		matches.push({
			module: flow.preprocessor_module,
			location: { type: 'preprocessor', index: -1 }
		})
	}

	visitFlowNodesInModules(flow.modules ?? [], (index) => ({ type: 'root', index }), (match) => {
		matches.push({
			module: match.module,
			location: match.location
		})
	})

	return matches
}

export function findModuleInModules(modules: FlowModule[], moduleId: string): FlowModule | undefined {
	return (
		findFlowNodeInModules(modules, moduleId, (index) => ({ type: 'root', index }))?.module ??
		undefined
	)
}

export function findFlowNode(flow: FlowModuleTree, moduleId: string): FlowNodeMatch | null {
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

	return findFlowNodeInModules(flow.modules ?? [], moduleId, (index) => ({ type: 'root', index }))
}

/**
 * Returns the live stored module for any module id in the flow tree.
 * This includes nested modules, AI-agent flowmodule tools, and special modules.
 */
export function findModuleInFlow(flow: FlowModuleTree, moduleId: string): FlowModule | null {
	return findFlowNode(flow, moduleId)?.module ?? null
}

export function findModuleParent(flow: FlowModuleTree, moduleId: string): ModuleParentLocation | null {
	return findFlowNode(flow, moduleId)?.location ?? null
}

function resolveModuleArrayByLocation(
	flow: FlowModuleTree,
	location: ArrayBackedModuleParentLocation,
	options: {
		createIfMissing?: boolean
		templateFlow?: FlowModuleTree
	} = {}
): FlowModule[] | null {
	if (location.type === 'root') {
		if (!flow.modules && options.createIfMissing) {
			flow.modules = []
		}
		return flow.modules ?? null
	}

	const parentModule = findModuleInFlow(flow, location.parentId)
	if (!parentModule) {
		return null
	}

	if (location.type === 'forloop' && parentModule.value.type === 'forloopflow') {
		return parentModule.value.modules ?? null
	}

	if (location.type === 'whileloop' && parentModule.value.type === 'whileloopflow') {
		return parentModule.value.modules ?? null
	}

	if (location.type === 'branchone-default' && parentModule.value.type === 'branchone') {
		return parentModule.value.default ?? null
	}

	if (location.type === 'branchone-branch' && parentModule.value.type === 'branchone') {
		let branch = parentModule.value.branches[location.branchIndex]
		if (!branch && options.templateFlow) {
			const templateParent = findModuleInFlow(options.templateFlow, location.parentId)
			const templateBranch =
				templateParent?.value.type === 'branchone'
					? templateParent.value.branches[location.branchIndex]
					: undefined
			if (templateBranch) {
				while (parentModule.value.branches.length <= location.branchIndex) {
					parentModule.value.branches.push({ expr: '', modules: [] })
				}
				parentModule.value.branches[location.branchIndex] = {
					...templateBranch,
					modules: []
				}
				branch = parentModule.value.branches[location.branchIndex]
			}
		}
		return branch?.modules ?? null
	}

	if (location.type === 'branchall-branch' && parentModule.value.type === 'branchall') {
		let branch = parentModule.value.branches[location.branchIndex]
		if (!branch && options.templateFlow) {
			const templateParent = findModuleInFlow(options.templateFlow, location.parentId)
			const templateBranch =
				templateParent?.value.type === 'branchall'
					? templateParent.value.branches[location.branchIndex]
					: undefined
			if (templateBranch) {
				while (parentModule.value.branches.length <= location.branchIndex) {
					parentModule.value.branches.push({ modules: [] })
				}
				parentModule.value.branches[location.branchIndex] = {
					...templateBranch,
					modules: []
				}
				branch = parentModule.value.branches[location.branchIndex]
			}
		}
		return branch?.modules ?? null
	}

	if (location.type === 'aiagent' && parentModule.value.type === 'aiagent') {
		if (!parentModule.value.tools && options.createIfMissing) {
			parentModule.value.tools = []
		}
		return (parentModule.value.tools as FlowModule[]) ?? null
	}

	return null
}

export function getModuleArrayByLocation(
	flow: FlowModuleTree,
	location: ArrayBackedModuleParentLocation
): FlowModule[] | null {
	return resolveModuleArrayByLocation(flow, location)
}

export function ensureModuleArrayByLocation(
	flow: FlowModuleTree,
	location: ArrayBackedModuleParentLocation,
	templateFlow?: FlowModuleTree
): FlowModule[] | null {
	return resolveModuleArrayByLocation(flow, location, {
		createIfMissing: true,
		templateFlow
	})
}

export function getModuleArrayContainer(
	flow: FlowModuleTree,
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

export function removeFlowModule(flow: FlowModuleTree, moduleId: string): FlowModule | null {
	if (flow.preprocessor_module?.id === moduleId) {
		const removed = flow.preprocessor_module
		flow.preprocessor_module = undefined
		return removed
	}

	if (flow.failure_module?.id === moduleId) {
		const removed = flow.failure_module
		flow.failure_module = undefined
		return removed
	}

	const match = findFlowNode(flow, moduleId)
	if (!match || !('container' in match)) {
		return null
	}

	const [removed] = match.container.splice(match.location.index, 1)
	return removed ?? null
}

export function replaceFlowModule(target: FlowModule, next: FlowModule): FlowModule {
	for (const key of Object.keys(target)) {
		delete (target as Record<string, unknown>)[key]
	}
	Object.assign(target, next)
	return target
}
