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

	// Build merged flow
	const mergedModules = items.map((item) => item.module)
	const isRemoved = (id: string | undefined) =>
		id !== undefined && (beforeActions[id] === 'removed' || beforeActions[id] === 'shadowed')

	const mergedFlow: FlowValue = {
		...afterFlow,
		modules: mergedModules,
		failure_module:
			afterFlow.failure_module ??
			(isRemoved(beforeFlow.failure_module?.id) ? beforeFlow.failure_module : undefined),
		preprocessor_module:
			afterFlow.preprocessor_module ??
			(isRemoved(beforeFlow.preprocessor_module?.id) ? beforeFlow.preprocessor_module : undefined)
	}

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
