import type { FlowModule, FlowValue } from '$lib/gen'
import { dfs } from './dfs'
import { deepEqual } from 'fast-equals'

/** Prefix added to module IDs when the original module coexists with a replacement */
export const DUPLICATE_MODULE_PREFIX = 'old__'

/** Prefix added to new module IDs when restoring original during type change rejection */
export const NEW_MODULE_PREFIX = 'new__'

/**
 * Action types for flow module changes during diff tracking
 * - added: Module was added to the flow
 * - modified: Module content was changed
 * - removed: Module was deleted from the flow
 * - shadowed: Module is shown as removed (visualization mode)
 */
export type AIModuleAction = 'added' | 'modified' | 'removed' | 'shadowed' | undefined

/**
 * Tracks the action performed on a module and whether it requires user approval
 */
export type ModuleActionInfo = {
	action: AIModuleAction
	/** Whether this change is pending user approval (accept/reject) */
	pending: boolean
}

/**
 * Normalizes a FlowModule for comparison by removing properties that
 * should be ignored when determining if a module has changed.
 * Specifically, removes empty `assets` arrays since their presence/absence
 * is not a meaningful difference. Recursively normalizes nested modules
 * within container types (branchone, branchall, forloopflow, whileloopflow, aiagent).
 */
function normalizeModuleForComparison(module: FlowModule): FlowModule {
	// Deep clone to avoid mutating the original and to handle nested structures
	const normalized = JSON.parse(JSON.stringify(module)) as FlowModule

	// Helper to remove empty assets from a value object
	function removeEmptyAssets(value: Record<string, unknown>): void {
		if (Array.isArray(value.assets) && value.assets.length === 0) {
			delete value.assets
		}
	}

	if ('value' in normalized && normalized.value && typeof normalized.value === 'object') {
		removeEmptyAssets(normalized.value as Record<string, unknown>)

		// Recursively normalize nested modules based on type
		const value = normalized.value
		if (value.type === 'forloopflow' || value.type === 'whileloopflow') {
			value.modules = value.modules.map((m) => normalizeModuleForComparison(m))
		} else if (value.type === 'branchone') {
			value.default = value.default.map((m) => normalizeModuleForComparison(m))
			value.branches = value.branches.map((branch) => ({
				...branch,
				modules: branch.modules.map((m) => normalizeModuleForComparison(m))
			}))
		} else if (value.type === 'branchall') {
			value.branches = value.branches.map((branch) => ({
				...branch,
				modules: branch.modules.map((m) => normalizeModuleForComparison(m))
			}))
		}
	}

	return normalized
}

/**
 * The complete diff result with action maps and merged flow
 */
export type FlowTimeline = {
	/** Actions for modules in the before flow */
	beforeActions: Record<string, ModuleActionInfo>

	/** Actions for modules in the after flow (adjusted based on display mode) */
	afterActions: Record<string, ModuleActionInfo>

	/** The merged flow containing both after modules and removed modules properly nested */
	mergedFlow: FlowValue
}

/**
 * Computes the difference between two flow versions and returns a map of module IDs to their actions.
 *
 * When a module exists in both flows but has a different type, it's treated as removed + added
 * rather than modified, since it's effectively a completely different module.
 *
 * @param beforeFlow - The original flow value
 * @param afterFlow - The modified flow value
 * @returns A record mapping module IDs to their diff result with separate before/after actions
 */
export function computeFlowModuleDiff(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	options: { markAsPending: boolean } = { markAsPending: false }
): {
	beforeActions: Record<string, ModuleActionInfo>
	afterActions: Record<string, ModuleActionInfo>
} {
	const beforeActions: Record<string, ModuleActionInfo> = {}
	const afterActions: Record<string, ModuleActionInfo> = {}

	// Get all modules with their locations from both flows
	const beforeModulesWithLoc = getAllModulesWithLocation(beforeFlow)
	const afterModulesWithLoc = getAllModulesWithLocation(afterFlow)

	// Find all module IDs
	const allModuleIds = new Set([...beforeModulesWithLoc.keys(), ...afterModulesWithLoc.keys()])

	for (const moduleId of allModuleIds) {
		const beforeEntry = beforeModulesWithLoc.get(moduleId)
		const afterEntry = afterModulesWithLoc.get(moduleId)

		if (!beforeEntry && afterEntry) {
			// Module exists in after but not before -> added
			afterActions[moduleId] = { action: 'added', pending: options.markAsPending }
		} else if (beforeEntry && !afterEntry) {
			// Module exists in before but not after -> removed
			beforeActions[moduleId] = { action: 'removed', pending: options.markAsPending }
			afterActions[moduleId] = { action: 'shadowed', pending: options.markAsPending }
		} else if (beforeEntry && afterEntry) {
			// Module exists in both -> check location first, then type and content
			if (!locationsEqual(beforeEntry.location, afterEntry.location)) {
				// Location changed -> treat as removed from old + added at new
				beforeActions[moduleId] = { action: 'removed', pending: options.markAsPending }
				afterActions[moduleId] = { action: 'added', pending: options.markAsPending }
			} else {
				// Same location -> check type and content
				const typeChanged = beforeEntry.module.value.type !== afterEntry.module.value.type
				if (typeChanged) {
					// Type changed -> treat as removed + added
					beforeActions[moduleId] = { action: 'removed', pending: options.markAsPending }
					afterActions[moduleId] = { action: 'added', pending: options.markAsPending }
				} else if (
					!deepEqual(
						normalizeModuleForComparison(beforeEntry.module),
						normalizeModuleForComparison(afterEntry.module)
					)
				) {
					// Same type but different content -> modified
					beforeActions[moduleId] = { action: 'modified', pending: options.markAsPending }
					afterActions[moduleId] = { action: 'modified', pending: options.markAsPending }
				}
			}
		}
	}

	return { beforeActions, afterActions }
}

/**
 * Helper function to get all modules from a flow as a Map
 */
function getAllModulesMap(flow: FlowValue): Map<string, FlowModule> {
	const moduleMap = new Map<string, FlowModule>()

	// Get all regular modules
	const allModules = dfs(flow.modules ?? [], (m) => m)
	for (const module of allModules) {
		if (module?.id) {
			moduleMap.set(module.id, module)
		}
	}

	// Add failure module if it exists
	if (flow.failure_module?.id) {
		moduleMap.set(flow.failure_module.id, flow.failure_module)
	}

	// Add preprocessor module if it exists
	if (flow.preprocessor_module?.id) {
		moduleMap.set(flow.preprocessor_module.id, flow.preprocessor_module)
	}

	return moduleMap
}

/**
 * Represents a module along with its location in the flow
 */
type ModuleWithLocation = {
	module: FlowModule
	location: ModuleParentLocation
}

/**
 * Helper function to get all modules from a flow as a Map with their locations
 */
function getAllModulesWithLocation(flow: FlowValue): Map<string, ModuleWithLocation> {
	const result = new Map<string, ModuleWithLocation>()
	const allModules = dfs(flow.modules ?? [], (m) => m)

	for (const module of allModules) {
		if (module?.id) {
			const location = findModuleParent(flow, module.id)
			if (location) {
				result.set(module.id, { module, location })
			}
		}
	}

	// Add special modules
	if (flow.failure_module?.id) {
		result.set(flow.failure_module.id, {
			module: flow.failure_module,
			location: { type: 'failure', index: -1 }
		})
	}
	if (flow.preprocessor_module?.id) {
		result.set(flow.preprocessor_module.id, {
			module: flow.preprocessor_module,
			location: { type: 'preprocessor', index: -1 }
		})
	}

	return result
}

/**
 * Compares two module locations for equality.
 * Two locations are equal if they refer to the same parent container.
 * Index within the container is not considered (modules can be reordered).
 */
export function locationsEqual(
	a: ModuleParentLocation | null,
	b: ModuleParentLocation | null
): boolean {
	if (!a || !b) return a === b
	if (a.type !== b.type) return false

	switch (a.type) {
		case 'root':
		case 'failure':
		case 'preprocessor':
			return true // Same type is enough (index doesn't matter for location equality)
		case 'forloop':
		case 'whileloop':
		case 'aiagent':
			return a.parentId === (b as typeof a).parentId
		case 'branchone-default':
			return a.parentId === (b as typeof a).parentId
		case 'branchone-branch':
		case 'branchall-branch':
			return (
				a.parentId === (b as typeof a).parentId && a.branchIndex === (b as typeof a).branchIndex
			)
		default:
			return false
	}
}

/**
 * Represents the parent location of a module
 */
export type ModuleParentLocation =
	| { type: 'root'; index: number }
	| { type: 'forloop' | 'whileloop'; parentId: string; index: number }
	| { type: 'branchone-default'; parentId: string; index: number }
	| { type: 'branchone-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'branchall-branch'; parentId: string; branchIndex: number; index: number }
	| { type: 'aiagent'; parentId: string; index: number }
	| { type: 'failure'; index: -1 }
	| { type: 'preprocessor'; index: -1 }

/**
 * Finds the parent location of a module in a flow
 */
export function findModuleParent(flow: FlowValue, moduleId: string): ModuleParentLocation | null {
	// Check special modules
	if (flow.failure_module?.id === moduleId) {
		return { type: 'failure', index: -1 }
	}
	if (flow.preprocessor_module?.id === moduleId) {
		return { type: 'preprocessor', index: -1 }
	}

	// Check root level
	const rootIndex = flow.modules?.findIndex((m) => m.id === moduleId)
	if (rootIndex !== undefined && rootIndex >= 0) {
		return { type: 'root', index: rootIndex }
	}

	// Recursively search nested modules
	function searchInModules(modules: FlowModule[]): ModuleParentLocation | null {
		for (const module of modules) {
			// Check forloopflow
			if (module.value.type === 'forloopflow') {
				const index = module.value.modules.findIndex((m) => m.id === moduleId)
				if (index >= 0) {
					return { type: 'forloop', parentId: module.id, index }
				}
				const nested = searchInModules(module.value.modules)
				if (nested) return nested
			}

			// Check whileloopflow
			if (module.value.type === 'whileloopflow') {
				const index = module.value.modules.findIndex((m) => m.id === moduleId)
				if (index >= 0) {
					return { type: 'whileloop', parentId: module.id, index }
				}
				const nested = searchInModules(module.value.modules)
				if (nested) return nested
			}

			// Check branchone
			if (module.value.type === 'branchone') {
				// Check default branch
				const defaultIndex = module.value.default.findIndex((m) => m.id === moduleId)
				if (defaultIndex >= 0) {
					return { type: 'branchone-default', parentId: module.id, index: defaultIndex }
				}
				const nestedDefault = searchInModules(module.value.default)
				if (nestedDefault) return nestedDefault

				// Check other branches
				for (let branchIndex = 0; branchIndex < module.value.branches.length; branchIndex++) {
					const branch = module.value.branches[branchIndex]
					const index = branch.modules.findIndex((m) => m.id === moduleId)
					if (index >= 0) {
						return { type: 'branchone-branch', parentId: module.id, branchIndex, index }
					}
					const nested = searchInModules(branch.modules)
					if (nested) return nested
				}
			}

			// Check branchall
			if (module.value.type === 'branchall') {
				for (let branchIndex = 0; branchIndex < module.value.branches.length; branchIndex++) {
					const branch = module.value.branches[branchIndex]
					const index = branch.modules.findIndex((m) => m.id === moduleId)
					if (index >= 0) {
						return { type: 'branchall-branch', parentId: module.id, branchIndex, index }
					}
					const nested = searchInModules(branch.modules)
					if (nested) return nested
				}
			}

			// Check aiagent
			if (module.value.type === 'aiagent' && module.value.tools) {
				const index = (module.value.tools as FlowModule[]).findIndex((m) => m.id === moduleId)
				if (index >= 0) {
					return { type: 'aiagent', parentId: module.id, index }
				}
				const nested = searchInModules(module.value.tools as FlowModule[])
				if (nested) return nested
			}
		}
		return null
	}

	return searchInModules(flow.modules ?? [])
}

/**
 * Deep clones a module to avoid mutation
 */
function cloneModule(module: FlowModule): FlowModule {
	return JSON.parse(JSON.stringify(module))
}

/**
 * Prepends a prefix to a module's ID and all nested child module IDs to avoid collisions
 */
function prependModuleId(module: FlowModule, prefix: string): FlowModule {
	const newModule = cloneModule(module)
	newModule.id = prefix + newModule.id

	// Recursively prefix nested module IDs
	if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
		newModule.value.modules = newModule.value.modules.map((m) => prependModuleId(m, prefix))
	} else if (newModule.value.type === 'branchone') {
		newModule.value.default = newModule.value.default.map((m) => prependModuleId(m, prefix))
		newModule.value.branches = newModule.value.branches.map((branch) => ({
			...branch,
			modules: branch.modules.map((m) => prependModuleId(m, prefix))
		}))
	} else if (newModule.value.type === 'branchall') {
		newModule.value.branches = newModule.value.branches.map((branch) => ({
			...branch,
			modules: branch.modules.map((m) => prependModuleId(m, prefix))
		}))
	} else if (newModule.value.type === 'aiagent' && newModule.value.tools) {
		// Handle aiagent tools - only prefix FlowModule tools, not MCP tools
		newModule.value.tools = newModule.value.tools.map((tool) => {
			// MCP tools have tool_type: 'mcp', FlowModule tools have tool_type: 'flowmodule' or undefined
			if (tool.value.tool_type === 'mcp') {
				return tool // MCP tools don't have nested module IDs
			}
			// For FlowModule tools, prefix the ID and recurse
			const prefixedTool = {
				...tool,
				id: prefix + tool.id
			}
			// If the tool has nested modules (it's a container type), recurse
			const innerValue = tool.value as FlowModule['value']
			if (innerValue.type === 'forloopflow' || innerValue.type === 'whileloopflow') {
				;(prefixedTool.value as any).modules = (innerValue as any).modules.map((m: FlowModule) =>
					prependModuleId(m, prefix)
				)
			} else if (innerValue.type === 'branchone') {
				;(prefixedTool.value as any).default = (innerValue as any).default.map((m: FlowModule) =>
					prependModuleId(m, prefix)
				)
				;(prefixedTool.value as any).branches = (innerValue as any).branches.map((branch: any) => ({
					...branch,
					modules: branch.modules.map((m: FlowModule) => prependModuleId(m, prefix))
				}))
			} else if (innerValue.type === 'branchall') {
				;(prefixedTool.value as any).branches = (innerValue as any).branches.map((branch: any) => ({
					...branch,
					modules: branch.modules.map((m: FlowModule) => prependModuleId(m, prefix))
				}))
			}
			return prefixedTool
		})
	}

	return newModule
}

/**
 * Collects all module IDs from a flow structure recursively
 */
function getAllModuleIds(flow: FlowValue): Set<string> {
	const ids = new Set<string>()

	function collectFromModules(modules: FlowModule[]): void {
		for (const module of modules) {
			if (module.id) {
				ids.add(module.id)
			}

			// Recursively collect from nested modules
			if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
				collectFromModules(module.value.modules)
			} else if (module.value.type === 'branchone') {
				collectFromModules(module.value.default)
				for (const branch of module.value.branches) {
					collectFromModules(branch.modules)
				}
			} else if (module.value.type === 'branchall') {
				for (const branch of module.value.branches) {
					collectFromModules(branch.modules)
				}
			} else if (module.value.type === 'aiagent' && module.value.tools) {
				collectFromModules(module.value.tools as FlowModule[])
			}
		}
	}

	// Collect from root modules
	if (flow.modules) {
		collectFromModules(flow.modules)
	}

	// Collect from special modules
	if (flow.failure_module?.id) {
		ids.add(flow.failure_module.id)
	}
	if (flow.preprocessor_module?.id) {
		ids.add(flow.preprocessor_module.id)
	}

	return ids
}

/**
 * Scans the merged flow for duplicate IDs and prefixes duplicates with 'old__'.
 * This handles the case where a module is moved from one location to another -
 * both the old and new versions end up in the merged flow with the same ID.
 */
function fixDuplicateIds(merged: FlowValue, beforeFlow: FlowValue): void {
	const seenIds = new Set<string>()
	const beforeModulesMap = getAllModulesMap(beforeFlow)

	// Process a single module - returns the (possibly prefixed) module
	function processModule(module: FlowModule): FlowModule {
		let result = module

		if (seenIds.has(module.id)) {
			// Duplicate found! Check if this one exists in beforeFlow
			const beforeModule = beforeModulesMap.get(module.id)
			if (beforeModule) {
				// This is the "old" version - prefix it and all its children
				result = prependModuleId(module, DUPLICATE_MODULE_PREFIX)
			}
		} else {
			seenIds.add(module.id)
		}

		// Recurse into nested modules (use result which may be prefixed)
		processNestedModules(result)

		return result
	}

	// Process nested modules in-place
	function processNestedModules(module: FlowModule): void {
		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			module.value.modules = module.value.modules.map((m) => processModule(m))
		} else if (module.value.type === 'branchone') {
			module.value.default = module.value.default.map((m) => processModule(m))
			for (const branch of module.value.branches) {
				branch.modules = branch.modules.map((m) => processModule(m))
			}
		} else if (module.value.type === 'branchall') {
			for (const branch of module.value.branches) {
				branch.modules = branch.modules.map((m) => processModule(m))
			}
		} else if (module.value.type === 'aiagent' && module.value.tools) {
			// For aiagent tools, we need to track IDs of FlowModule tools
			for (const tool of module.value.tools) {
				if (tool.value.tool_type !== 'mcp') {
					if (seenIds.has(tool.id)) {
						// Can't easily prefix in-place here, but aiagent tools rarely move
						// The main use case is regular modules moving in/out of loops/branches
					} else {
						seenIds.add(tool.id)
					}
				}
			}
		}
	}

	// Process root modules
	if (merged.modules) {
		merged.modules = merged.modules.map((m) => processModule(m))
	}

	// Process special modules
	if (merged.failure_module) {
		if (seenIds.has(merged.failure_module.id)) {
			const beforeModule = beforeModulesMap.get(merged.failure_module.id)
			if (beforeModule) {
				merged.failure_module = prependModuleId(merged.failure_module, DUPLICATE_MODULE_PREFIX)
			}
		} else {
			seenIds.add(merged.failure_module.id)
		}
	}

	if (merged.preprocessor_module) {
		if (seenIds.has(merged.preprocessor_module.id)) {
			const beforeModule = beforeModulesMap.get(merged.preprocessor_module.id)
			if (beforeModule) {
				merged.preprocessor_module = prependModuleId(
					merged.preprocessor_module,
					DUPLICATE_MODULE_PREFIX
				)
			}
		} else {
			seenIds.add(merged.preprocessor_module.id)
		}
	}
}

/**
 * Reconstructs the merged flow with removed modules properly nested
 */
function reconstructMergedFlow(
	afterFlow: FlowValue,
	beforeFlow: FlowValue,
	beforeActions: Record<string, ModuleActionInfo>
): FlowValue {
	// Deep clone afterFlow to avoid mutation
	const merged: FlowValue = JSON.parse(JSON.stringify(afterFlow))

	// Get all removed/shadowed modules from beforeFlow
	const removedModules = Object.entries(beforeActions)
		.filter(([_, action]) => action.action === 'removed' || action.action === 'shadowed')
		.map(([id]) => id)

	// Create a Set for faster lookup
	const removedModulesSet = new Set(removedModules)

	// Cache beforeFlow modules map and merged IDs to avoid recomputing in the loop
	const beforeModulesMap = getAllModulesMap(beforeFlow)
	const mergedIds = getAllModuleIds(merged)

	// For each removed module, find its parent and insert it
	for (const removedId of removedModules) {
		const beforeModule = beforeModulesMap.get(removedId)
		if (!beforeModule) continue

		const parentLocation = findModuleParent(beforeFlow, removedId)
		if (!parentLocation) continue

		// Skip if parent is also removed - the module will be inserted as part of its parent
		// This prevents duplicates when removing container modules with nested children
		if (
			parentLocation.type !== 'root' &&
			parentLocation.type !== 'failure' &&
			parentLocation.type !== 'preprocessor'
		) {
			if (removedModulesSet.has(parentLocation.parentId)) {
				// Parent is also removed, skip this module
				continue
			}
		}

		let clonedModule = cloneModule(beforeModule)

		// Check for ID collision - this happens when a module type changed
		// In this case, the new module is already in the merged flow as 'added'
		// We prepend the duplicate prefix to the removed module's ID so both can coexist
		if (mergedIds.has(clonedModule.id)) {
			clonedModule = prependModuleId(clonedModule, DUPLICATE_MODULE_PREFIX)
		}

		// Track the newly added module ID
		mergedIds.add(clonedModule.id)

		// Insert based on parent location
		if (parentLocation.type === 'failure') {
			merged.failure_module = clonedModule
		} else if (parentLocation.type === 'preprocessor') {
			merged.preprocessor_module = clonedModule
		} else if (parentLocation.type === 'root') {
			// Find the best position to insert in root modules
			const insertIndex = findBestInsertPosition(
				merged.modules ?? [],
				beforeFlow.modules ?? [],
				parentLocation.index,
				removedId
			)
			if (!merged.modules) merged.modules = []
			merged.modules.splice(insertIndex, 0, clonedModule)
		} else {
			// Find the parent module in merged flow and insert into it
			insertIntoNestedParent(merged, parentLocation, clonedModule, beforeFlow)
		}
	}

	// Post-process: fix any duplicate IDs that may have been created
	// This handles the case where a module moved from one location to another
	fixDuplicateIds(merged, beforeFlow)

	return merged
}

/**
 * Finds the best position to insert a removed module in a module array
 */
function findBestInsertPosition(
	targetModules: FlowModule[],
	beforeModules: FlowModule[],
	originalIndex: number,
	removedId: string
): number {
	// Look for anchors (modules that exist in both before and after)
	// Try to find previous anchor
	for (let i = originalIndex - 1; i >= 0; i--) {
		const anchorId = beforeModules[i]?.id
		const anchorIndex = targetModules.findIndex((m) => m.id === anchorId)
		if (anchorIndex >= 0) {
			return anchorIndex + 1
		}
	}

	// Try to find next anchor
	for (let i = originalIndex + 1; i < beforeModules.length; i++) {
		const anchorId = beforeModules[i]?.id
		const anchorIndex = targetModules.findIndex((m) => m.id === anchorId)
		if (anchorIndex >= 0) {
			return anchorIndex
		}
	}

	// No anchors found, append to end
	return targetModules.length
}

/**
 * Inserts a removed module into its nested parent in the merged flow
 */
function insertIntoNestedParent(
	merged: FlowValue,
	parentLocation: ModuleParentLocation,
	moduleToInsert: FlowModule,
	beforeFlow: FlowValue
): void {
	if (
		parentLocation.type === 'root' ||
		parentLocation.type === 'failure' ||
		parentLocation.type === 'preprocessor'
	) {
		return
	}

	// Find the parent module in merged flow
	const parentModule = findModuleById(merged, parentLocation.parentId)
	if (!parentModule) {
		console.warn('Parent module not found', parentLocation)
		return
	}

	// Get the before parent to know original ordering
	const beforeParent = findModuleById(beforeFlow, parentLocation.parentId)
	if (!beforeParent) {
		console.warn('Before parent module not found', parentLocation)
		return
	}

	// Insert based on type
	if (parentLocation.type === 'forloop' && parentModule.value.type === 'forloopflow') {
		const beforeModules = (beforeParent.value as any).modules ?? []
		const insertIndex = findBestInsertPosition(
			parentModule.value.modules,
			beforeModules,
			parentLocation.index,
			moduleToInsert.id
		)
		parentModule.value.modules.splice(insertIndex, 0, moduleToInsert)
	} else if (parentLocation.type === 'whileloop' && parentModule.value.type === 'whileloopflow') {
		const beforeModules = (beforeParent.value as any).modules ?? []
		const insertIndex = findBestInsertPosition(
			parentModule.value.modules,
			beforeModules,
			parentLocation.index,
			moduleToInsert.id
		)
		parentModule.value.modules.splice(insertIndex, 0, moduleToInsert)
	} else if (
		parentLocation.type === 'branchone-default' &&
		parentModule.value.type === 'branchone'
	) {
		const beforeModules = (beforeParent.value as any).default ?? []
		const insertIndex = findBestInsertPosition(
			parentModule.value.default,
			beforeModules,
			parentLocation.index,
			moduleToInsert.id
		)
		parentModule.value.default.splice(insertIndex, 0, moduleToInsert)
	} else if (
		parentLocation.type === 'branchone-branch' &&
		parentModule.value.type === 'branchone'
	) {
		let branch = parentModule.value.branches[parentLocation.branchIndex]
		const beforeBranch = (beforeParent.value as any).branches?.[parentLocation.branchIndex]

		// If the branch doesn't exist (entire branch was removed), recreate it from beforeFlow
		if (!branch && beforeBranch) {
			// Ensure we have enough branch slots
			while (parentModule.value.branches.length <= parentLocation.branchIndex) {
				parentModule.value.branches.push({ expr: '', modules: [] })
			}
			// Restore the branch with its original expr but empty modules (we'll add them)
			parentModule.value.branches[parentLocation.branchIndex] = {
				...beforeBranch,
				modules: []
			}
			branch = parentModule.value.branches[parentLocation.branchIndex]
		}

		if (branch) {
			const beforeModules = beforeBranch?.modules ?? []
			const insertIndex = findBestInsertPosition(
				branch.modules,
				beforeModules,
				parentLocation.index,
				moduleToInsert.id
			)
			branch.modules.splice(insertIndex, 0, moduleToInsert)
		}
	} else if (
		parentLocation.type === 'branchall-branch' &&
		parentModule.value.type === 'branchall'
	) {
		let branch = parentModule.value.branches[parentLocation.branchIndex]
		const beforeBranch = (beforeParent.value as any).branches?.[parentLocation.branchIndex]

		// If the branch doesn't exist (entire branch was removed), recreate it from beforeFlow
		if (!branch && beforeBranch) {
			// Ensure we have enough branch slots
			while (parentModule.value.branches.length <= parentLocation.branchIndex) {
				parentModule.value.branches.push({ modules: [] })
			}
			// Restore the branch with empty modules (we'll add them)
			parentModule.value.branches[parentLocation.branchIndex] = {
				...beforeBranch,
				modules: []
			}
			branch = parentModule.value.branches[parentLocation.branchIndex]
		}

		if (branch) {
			const beforeModules = beforeBranch?.modules ?? []
			const insertIndex = findBestInsertPosition(
				branch.modules,
				beforeModules,
				parentLocation.index,
				moduleToInsert.id
			)
			branch.modules.splice(insertIndex, 0, moduleToInsert)
		}
	} else if (parentLocation.type === 'aiagent' && parentModule.value.type === 'aiagent') {
		const tools = (parentModule.value.tools as FlowModule[]) ?? []
		const beforeTools = ((beforeParent.value as any).tools as FlowModule[]) ?? []
		const insertIndex = findBestInsertPosition(
			tools,
			beforeTools,
			parentLocation.index,
			moduleToInsert.id
		)
		tools.splice(insertIndex, 0, moduleToInsert)
	}
}

/**
 * Finds a module by ID anywhere in the flow
 */
function findModuleById(flow: FlowValue, moduleId: string): FlowModule | null {
	const moduleMap = getAllModulesMap(flow)
	return moduleMap.get(moduleId) ?? null
}

/**
 * Adjusts the after actions based on display mode and adds entries for prefixed IDs
 */
function adjustActionsForDisplay(
	afterActions: Record<string, ModuleActionInfo>,
	beforeActions: Record<string, ModuleActionInfo>,
	markRemovedAsShadowed: boolean,
	mergedFlow: FlowValue
): Record<string, ModuleActionInfo> {
	const adjusted: Record<string, ModuleActionInfo> = {}

	// Copy all existing actions
	for (const [id, action] of Object.entries(afterActions)) {
		if (!markRemovedAsShadowed && action.action === 'shadowed') {
			// In unified mode, change 'shadowed' to 'removed' for proper coloring
			adjusted[id] = { action: 'removed', pending: action.pending }
		} else {
			adjusted[id] = action
		}
	}

	// Add entries for prefixed IDs (modules that had type changes or were removed)
	// These are the old versions that got the duplicate prefix prepended to their ID
	const allMergedIds = getAllModuleIds(mergedFlow)
	for (const id of allMergedIds) {
		if (id.startsWith(DUPLICATE_MODULE_PREFIX) && !adjusted[id]) {
			// This is a prefixed ID for a module that was removed
			const originalId = id.substring(DUPLICATE_MODULE_PREFIX.length)
			// Check beforeActions to see if this module was removed
			if (beforeActions[originalId]?.action === 'removed') {
				adjusted[id] = {
					action: markRemovedAsShadowed ? 'shadowed' : 'removed',
					pending: beforeActions[originalId].pending
				}
			}
		}
	}

	return adjusted
}

/**
 * Builds the complete flow diff result with action maps and merged flow.
 * The merged flow contains all modules from afterFlow plus removed modules from
 * beforeFlow properly nested in their original locations.
 *
 * @param beforeFlow - The original flow value
 * @param afterFlow - The modified flow value
 * @param options - Display options
 * @returns Complete diff result with beforeActions, afterActions, and mergedFlow
 */
export function buildFlowTimeline(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	options: { markRemovedAsShadowed: boolean; markAsPending: boolean } = {
		markRemovedAsShadowed: false,
		markAsPending: false
	}
): FlowTimeline {
	// Compute the diff between the two flows
	const { beforeActions, afterActions } = computeFlowModuleDiff(beforeFlow, afterFlow, {
		markAsPending: options.markAsPending
	})

	// Reconstruct merged flow with removed modules properly nested
	const mergedFlow = reconstructMergedFlow(afterFlow, beforeFlow, beforeActions)

	// Adjust after actions based on display mode and add entries for prefixed IDs
	const adjustedAfterActions = adjustActionsForDisplay(
		afterActions,
		beforeActions,
		options.markRemovedAsShadowed,
		mergedFlow
	)

	return {
		beforeActions,
		afterActions: adjustedAfterActions,
		mergedFlow
	}
}

/**
 * Inserts a module into a flow at its correct position based on where it was located in the source flow.
 * This is useful when restoring a removed module - it finds the correct parent and position.
 *
 * @param targetFlow - The flow to insert the module into
 * @param moduleToInsert - The module to insert
 * @param sourceFlow - The flow where the module originally existed (to find parent location and ordering)
 * @param moduleId - The ID of the module being inserted
 */
export function insertModuleIntoFlow(
	targetFlow: FlowValue,
	moduleToInsert: FlowModule,
	sourceFlow: FlowValue,
	moduleId: string
): void {
	const parentLocation = findModuleParent(sourceFlow, moduleId)
	if (!parentLocation) return

	// Handle special modules
	if (parentLocation.type === 'failure') {
		targetFlow.failure_module = moduleToInsert
		return
	}
	if (parentLocation.type === 'preprocessor') {
		targetFlow.preprocessor_module = moduleToInsert
		return
	}

	// Handle root level modules
	if (parentLocation.type === 'root') {
		const insertIndex = findBestInsertPosition(
			targetFlow.modules ?? [],
			sourceFlow.modules ?? [],
			parentLocation.index,
			moduleId
		)
		if (!targetFlow.modules) targetFlow.modules = []
		targetFlow.modules.splice(insertIndex, 0, moduleToInsert)
		return
	}

	// Handle nested modules
	insertIntoNestedParent(targetFlow, parentLocation, moduleToInsert, sourceFlow)
}

/**
 * Finds a module by ID anywhere in a flow (including nested modules, failure, and preprocessor)
 *
 * @param flow - The flow to search in
 * @param moduleId - The ID of the module to find
 * @returns The module if found, null otherwise
 */
export function findModuleInFlow(flow: FlowValue, moduleId: string): FlowModule | null {
	return findModuleById(flow, moduleId)
}

/**
 * Checks if the input schema has changed between two flow versions.
 * The input schema always exists (even if empty), so we only check for modifications.
 *
 * @param beforeFlow - The original flow (can be OpenFlow or just have schema property)
 * @param afterFlow - The modified flow (can be OpenFlow or just have schema property)
 * @returns true if the schemas are different, false if identical
 */
export function hasInputSchemaChanged(
	beforeFlow: { schema?: { [key: string]: unknown } } | undefined,
	afterFlow: { schema?: { [key: string]: unknown } } | undefined
): boolean {
	if (!beforeFlow || !afterFlow) {
		return false
	}

	return !deepEqual(beforeFlow.schema, afterFlow.schema)
}
