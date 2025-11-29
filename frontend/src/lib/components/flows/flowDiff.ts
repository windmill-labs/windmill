import type { FlowModule, FlowValue } from '$lib/gen'
import { dfs } from './dfs'
import { deepEqual } from 'fast-equals'
import type { ModuleActionInfo } from '../copilot/chat/flow/core'

/** Prefix added to module IDs when the original module coexists with a replacement */
export const DUPLICATE_MODULE_PREFIX = 'old__'

/**
 * Normalizes a FlowModule for comparison by removing properties that
 * should be ignored when determining if a module has changed.
 * Specifically, removes empty `assets` arrays since their presence/absence
 * is not a meaningful difference.
 */
function normalizeModuleForComparison(module: FlowModule): FlowModule {
	const normalized = { ...module }
	if ('value' in normalized && normalized.value && typeof normalized.value === 'object') {
		const value = { ...normalized.value } as Record<string, unknown>
		// Remove empty assets array - it's not a meaningful difference
		if (Array.isArray(value.assets) && value.assets.length === 0) {
			delete value.assets
		}
		normalized.value = value as FlowModule['value']
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

	// Get all modules from both flows using dfs
	const beforeModules = getAllModulesMap(beforeFlow)
	const afterModules = getAllModulesMap(afterFlow)

	// Find all module IDs
	const allModuleIds = new Set([...beforeModules.keys(), ...afterModules.keys()])

	for (const moduleId of allModuleIds) {
		const beforeModule = beforeModules.get(moduleId)
		const afterModule = afterModules.get(moduleId)

		if (!beforeModule && afterModule) {
			// Module exists in after but not before -> added
			afterActions[moduleId] = { action: 'added', pending: options.markAsPending }
		} else if (beforeModule && !afterModule) {
			// Module exists in before but not after -> removed
			beforeActions[moduleId] = { action: 'removed', pending: options.markAsPending }
			afterActions[moduleId] = { action: 'shadowed', pending: options.markAsPending }
		} else if (beforeModule && afterModule) {
			// Module exists in both -> check type and content
			const typeChanged = beforeModule.value.type !== afterModule.value.type
			if (typeChanged) {
				// Type changed -> treat as removed + added
				beforeActions[moduleId] = { action: 'removed', pending: options.markAsPending }
				afterActions[moduleId] = { action: 'added', pending: options.markAsPending }
			} else if (!deepEqual(normalizeModuleForComparison(beforeModule), normalizeModuleForComparison(afterModule))) {
				// Same type but different content -> modified
				beforeActions[moduleId] = { action: 'modified', pending: options.markAsPending }
				afterActions[moduleId] = { action: 'modified', pending: options.markAsPending }
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
 * Prepends a prefix to a module's ID to avoid collisions
 */
function prependModuleId(module: FlowModule, prefix: string): FlowModule {
	const newModule = cloneModule(module)
	newModule.id = prefix + newModule.id
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

	// For each removed module, find its parent and insert it
	for (const removedId of removedModules) {
		const beforeModule = getAllModulesMap(beforeFlow).get(removedId)
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
		const existingIds = getAllModuleIds(merged)
		if (existingIds.has(clonedModule.id)) {
			clonedModule = prependModuleId(clonedModule, DUPLICATE_MODULE_PREFIX)
		}

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
		const branch = parentModule.value.branches[parentLocation.branchIndex]
		if (branch) {
			const beforeBranch = (beforeParent.value as any).branches?.[parentLocation.branchIndex]
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
		const branch = parentModule.value.branches[parentLocation.branchIndex]
		if (branch) {
			const beforeBranch = (beforeParent.value as any).branches?.[parentLocation.branchIndex]
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
