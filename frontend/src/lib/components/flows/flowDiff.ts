import type { FlowModule, FlowValue } from '$lib/gen'
import { dfs } from './dfs'
import { deepEqual } from 'fast-equals'
import type { AIModuleAction } from '../copilot/chat/flow/core'

/**
 * Represents a single item in the merged timeline for visualization.
 * Each item has complete context about its history and operation type.
 */
export type TimelineItem = {
	/** The flow module itself */
	module: FlowModule

	/** Sequential position in the final timeline (0, 1, 2, ...) */
	position: number

	/** The operation that occurred to this module (undefined for unchanged modules) */
	operation: AIModuleAction | undefined

	/** Position in beforeFlow.modules (undefined if didn't exist) */
	beforeIndex?: number

	/** Position in afterFlow.modules (undefined if doesn't exist) */
	afterIndex?: number

	/** Helps track insertion logic */
	insertionStrategy?: 'original_position' | 'anchor_based' | 'end_of_flow'
}

/**
 * The complete timeline result with metadata
 */
export type FlowTimeline = {
	/** Ordered list of all modules in rendering order */
	items: TimelineItem[]

	/** The before actions */
	beforeActions: Record<string, AIModuleAction>

	/** The merged flow for compatibility with existing code */
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
	afterFlow: FlowValue
): { beforeActions: Record<string, AIModuleAction>; afterActions: Record<string, AIModuleAction> } {
	const beforeActions: Record<string, AIModuleAction> = {}
	const afterActions: Record<string, AIModuleAction> = {}

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
			afterActions[moduleId] = 'added'
		} else if (beforeModule && !afterModule) {
			// Module exists in before but not after -> removed
			beforeActions[moduleId] = 'removed'
			afterActions[moduleId] = 'shadowed'
		} else if (beforeModule && afterModule) {
			// Module exists in both -> check type and content
			const typeChanged = beforeModule.value.type !== afterModule.value.type
			if (typeChanged) {
				// Type changed -> treat as removed + added
				beforeActions[moduleId] = 'removed'
				afterActions[moduleId] = 'added'
			} else if (!deepEqual(beforeModule, afterModule)) {
				// Same type but different content -> modified
				beforeActions[moduleId] = 'modified'
				afterActions[moduleId] = 'modified'
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
 * Helper function to get the indices of top-level modules
 */
function getTopLevelModuleIndices(flow: FlowValue): Map<string, number> {
	const indexMap = new Map<string, number>()

	// Only track top-level modules
	;(flow.modules ?? []).forEach((module, index) => {
		if (module?.id) {
			indexMap.set(module.id, index)
		}
	})

	return indexMap
}

/**
 * Represents the parent location of a module
 */
type ModuleParentLocation =
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
function findModuleParent(flow: FlowValue, moduleId: string): ModuleParentLocation | null {
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
 * Prepends a prefix to a module's ID
 */
function prependModuleId(module: FlowModule, prefix: string): FlowModule {
	const newModule = cloneModule(module)
	newModule.id = prefix + newModule.id
	return newModule
}

/**
 * Reconstructs the merged flow with removed modules properly nested
 */
function reconstructMergedFlow(
	afterFlow: FlowValue,
	beforeFlow: FlowValue,
	beforeActions: Record<string, AIModuleAction>
): FlowValue {
	// Deep clone afterFlow to avoid mutation
	const merged: FlowValue = JSON.parse(JSON.stringify(afterFlow))

	// Get all removed/shadowed modules from beforeFlow
	const removedModules = Object.entries(beforeActions)
		.filter(([_, action]) => action === 'removed' || action === 'shadowed')
		.map(([id]) => id)

	// For each removed module, find its parent and insert it
	for (const removedId of removedModules) {
		const beforeModule = getAllModulesMap(beforeFlow).get(removedId)
		if (!beforeModule) continue

		const parentLocation = findModuleParent(beforeFlow, removedId)
		if (!parentLocation) continue

		let clonedModule = cloneModule(beforeModule)

		// Check for ID collision and prepend "__" if necessary
		const existingIds = getAllModuleIds(merged)
		if (existingIds.has(clonedModule.id)) {
			clonedModule = prependModuleId(clonedModule, '__')
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
	if (!parentModule) return

	// Get the before parent to know original ordering
	const beforeParent = findModuleById(beforeFlow, parentLocation.parentId)
	if (!beforeParent) return

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
 * Builds a sequential timeline of flow modules for diff visualization.
 * Uses afterFlow positions as baseline and inserts removed modules using anchor-based strategy.
 * This ensures that the final positions of modules are preserved correctly.
 */
export function buildFlowTimeline(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	options: { markRemovedAsShadowed: boolean } = { markRemovedAsShadowed: false }
): FlowTimeline {
	const beforeModules = getAllModulesMap(beforeFlow)
	const afterModules = getAllModulesMap(afterFlow)
	const beforeIndices = getTopLevelModuleIndices(beforeFlow)

	const { beforeActions, afterActions } = computeFlowModuleDiff(beforeFlow, afterFlow)

	// Build timeline items starting with afterFlow order (preserves final positions)
	const timelineItems: TimelineItem[] = []

	// First, add all modules from afterFlow in their final order
	const afterFlowModules = (afterFlow.modules ?? [])
		.map((m, idx) => ({ module: m, afterIndex: idx }))
		.sort((a, b) => a.afterIndex - b.afterIndex)

	for (const { module: afterModule, afterIndex } of afterFlowModules) {
		const moduleId = afterModule.id
		const beforeIndex = beforeIndices.get(moduleId)

		// Determine operation - undefined means unchanged module
		const operation: AIModuleAction | undefined = afterActions[moduleId]

		timelineItems.push({
			module: afterModule,
			position: afterIndex,
			operation,
			beforeIndex,
			afterIndex,
			insertionStrategy: 'original_position'
		})
	}

	// Then, insert removed modules using anchor-based strategy
	const removedModules = Array.from(beforeModules.keys())
		.filter((id) => !afterModules.has(id))
		.map((id) => ({
			id,
			module: beforeModules.get(id)!,
			beforeIndex: beforeIndices.get(id)
		}))
		.sort((a, b) => (a.beforeIndex ?? 0) - (b.beforeIndex ?? 0))

	for (const removed of removedModules) {
		const insertPoint = findRemovedModulePosition(removed, timelineItems, beforeFlow.modules ?? [])

		const operation = options.markRemovedAsShadowed ? 'shadowed' : 'removed'

		timelineItems.splice(insertPoint.index, 0, {
			module: removed.module,
			position: insertPoint.index,
			operation,
			beforeIndex: removed.beforeIndex,
			afterIndex: undefined,
			insertionStrategy: insertPoint.strategy
		})
	}

	// Finalize positions
	const items = timelineItems.map((item, index) => ({ ...item, position: index }))

	// Build merged flow with proper nesting of removed modules
	const mergedFlow = reconstructMergedFlow(afterFlow, beforeFlow, beforeActions)

	return { items, beforeActions, mergedFlow }
}

/**
 * Finds insertion point for a removed module by looking at its neighbors in beforeFlow.
 * Uses anchor-based strategy to place removed modules near their original context.
 */
function findRemovedModulePosition(
	removedModule: { id: string; beforeIndex?: number; module: FlowModule },
	timelineItems: TimelineItem[],
	beforeFlowModules: FlowModule[]
): { index: number; strategy: 'original_position' | 'anchor_based' | 'end_of_flow' } {
	if (removedModule.beforeIndex === undefined) {
		return { index: timelineItems.length, strategy: 'end_of_flow' }
	}

	// Find nearest anchor module (from beforeFlow that still exists) before and after this position
	let prevAnchor: string | undefined
	for (let i = removedModule.beforeIndex - 1; i >= 0; i--) {
		const id = beforeFlowModules[i]?.id
		if (
			timelineItems.find(
				(item) => item.module.id === id && item.insertionStrategy === 'original_position'
			)
		) {
			prevAnchor = id
			break
		}
	}

	let nextAnchor: string | undefined
	for (let i = removedModule.beforeIndex + 1; i < beforeFlowModules.length; i++) {
		const id = beforeFlowModules[i]?.id
		if (
			timelineItems.find(
				(item) => item.module.id === id && item.insertionStrategy === 'original_position'
			)
		) {
			nextAnchor = id
			break
		}
	}

	// Insert between anchors, preferring to place after the previous anchor
	if (prevAnchor) {
		const prevPos = timelineItems.findIndex((item) => item.module.id === prevAnchor)
		return { index: prevPos + 1, strategy: 'anchor_based' }
	} else if (nextAnchor) {
		const nextPos = timelineItems.findIndex((item) => item.module.id === nextAnchor)
		return { index: nextPos, strategy: 'anchor_based' }
	} else {
		// If no anchors found, try to maintain relative ordering with other removed modules
		// by finding the best position based on beforeIndex
		let insertIndex = 0
		for (let i = 0; i < timelineItems.length; i++) {
			const item = timelineItems[i]
			if (item.beforeIndex !== undefined && item.beforeIndex < removedModule.beforeIndex) {
				insertIndex = i + 1
			} else {
				break
			}
		}
		return { index: insertIndex, strategy: 'anchor_based' }
	}
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
