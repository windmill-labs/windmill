import type { FlowModule, FlowValue } from '$lib/gen'
import { dfs } from './dfs'
import { deepEqual } from 'fast-equals'
import type { AIModuleAction } from '../copilot/chat/flow/core'

/**
 * Represents the diff actions for a single module, potentially different for before/after views.
 */
export type ModuleDiffResult = {
	before?: AIModuleAction
	after?: AIModuleAction
	/** Original index in beforeFlow.modules for removed top-level modules */
	originalIndex?: number
}

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

	/** If the module ID had a conflict and was renamed */
	renamedFrom?: string

	/** Helps track insertion logic */
	insertionStrategy?: 'original_position' | 'anchor_based' | 'end_of_flow'
}

/**
 * The complete timeline result with metadata
 */
export type FlowTimeline = {
	/** Ordered list of all modules in rendering order */
	items: TimelineItem[]

	/** Mapping from module ID to timeline item for quick lookup */
	itemMap: Map<string, TimelineItem>

	/** Statistics about the diff */
	stats: {
		added: number
		removed: number
		modified: number
		kept: number
		shadowed: number
	}

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
): Record<string, ModuleDiffResult> {
	const result: Record<string, ModuleDiffResult> = {}

	// Get all modules from both flows using dfs
	const beforeModules = getAllModulesMap(beforeFlow)
	const afterModules = getAllModulesMap(afterFlow)

	// Get top-level indices for removed modules
	const beforeIndices = getTopLevelModuleIndices(beforeFlow)

	// Find all module IDs
	const allModuleIds = new Set([...beforeModules.keys(), ...afterModules.keys()])

	for (const moduleId of allModuleIds) {
		const beforeModule = beforeModules.get(moduleId)
		const afterModule = afterModules.get(moduleId)

		if (!beforeModule && afterModule) {
			// Module exists in after but not before -> added
			result[moduleId] = { after: 'added' }
		} else if (beforeModule && !afterModule) {
			// Module exists in before but not after -> removed
			const originalIndex = beforeIndices.get(moduleId)
			result[moduleId] = { before: 'removed', originalIndex }
		} else if (beforeModule && afterModule) {
			// Module exists in both -> check type and content
			const typeChanged = beforeModule.value.type !== afterModule.value.type

			if (typeChanged) {
				// Type changed -> treat as removed + added
				const originalIndex = beforeIndices.get(moduleId)
				result[moduleId] = { before: 'removed', after: 'added', originalIndex }
			} else if (!deepEqual(beforeModule, afterModule)) {
				// Same type but different content -> modified
				result[moduleId] = { before: 'modified', after: 'modified' }
			}
			// If they're equal, don't add to result (no change)
		}
	}

	return result
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
 * Splits a module diff map into separate maps for before and after views.
 * - Before view: shows modules with before actions (removed or modified)
 * - After view: shows modules with after actions (added or modified)
 *
 * @param moduleDiff - The complete module diff with separate before/after actions
 * @returns An object with beforeActions and afterActions maps
 */
export function splitModuleDiffForViews(moduleDiff: Record<string, ModuleDiffResult>): {
	beforeActions: Record<string, AIModuleAction>
	afterActions: Record<string, AIModuleAction>
} {
	const beforeActions: Record<string, AIModuleAction> = {}
	const afterActions: Record<string, AIModuleAction> = {}

	for (const [moduleId, diffResult] of Object.entries(moduleDiff)) {
		if (diffResult.before) {
			beforeActions[moduleId] = diffResult.before
		}
		if (diffResult.after) {
			afterActions[moduleId] = diffResult.after
		}
	}

	return { beforeActions, afterActions }
}

/**
 * Builds a sequential timeline of flow modules for diff visualization.
 * Uses beforeFlow positions as baseline and inserts added modules using anchor-based strategy.
 */
export function buildFlowTimeline(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>,
	options: { markRemovedAsShadowed: boolean } = { markRemovedAsShadowed: false }
): FlowTimeline {
	const beforeModules = getAllModulesMap(beforeFlow)
	const afterModules = getAllModulesMap(afterFlow)
	const afterIndices = getTopLevelModuleIndices(afterFlow)
	const allModuleIds = new Set([...beforeModules.keys(), ...afterModules.keys()])
	const afterModuleIds = new Set(afterModules.keys())

	// Build timeline items starting with beforeFlow order
	const timelineItems: TimelineItem[] = []

	// First, add all modules from beforeFlow in their original order
	const beforeFlowModules = (beforeFlow.modules ?? [])
		.map((m, idx) => ({ module: m, beforeIndex: idx }))
		.sort((a, b) => a.beforeIndex - b.beforeIndex)

	for (const { module: beforeModule, beforeIndex } of beforeFlowModules) {
		const moduleId = beforeModule.id
		const afterModule = afterModules.get(moduleId)
		const afterIndex = afterIndices.get(moduleId)

		// Determine operation - undefined means unchanged module
		let operation: AIModuleAction | undefined =
			moduleDiff[moduleId]?.after ?? moduleDiff[moduleId]?.before
		if (!afterModule) {
			operation = options.markRemovedAsShadowed ? 'shadowed' : 'removed'
		}

		// Handle ID conflict (module removed but same ID exists in after with different content)
		let actualModule =
			afterModule && operation !== 'removed' && operation !== 'shadowed'
				? afterModule
				: beforeModule
		let actualId = moduleId
		let renamedFrom: string | undefined

		if (!afterModule && afterModuleIds.has(moduleId)) {
			actualId = `__removed__${moduleId}`
			actualModule = { ...beforeModule, id: actualId }
			renamedFrom = moduleId
		}

		timelineItems.push({
			module: actualModule,
			position: beforeIndex,
			operation,
			beforeIndex,
			afterIndex,
			renamedFrom,
			insertionStrategy: 'original_position'
		})
	}

	// Then, insert added modules using anchor-based strategy
	const addedModules = Array.from(allModuleIds)
		.filter((id) => !beforeModules.has(id) && afterModules.has(id))
		.map((id) => ({
			id,
			module: afterModules.get(id)!,
			afterIndex: afterIndices.get(id)
		}))

	for (const added of addedModules) {
		const insertPoint = findInsertionPoint(added, timelineItems, afterFlow.modules ?? [])

		timelineItems.splice(insertPoint.index, 0, {
			module: added.module,
			position: insertPoint.index,
			operation: 'added',
			beforeIndex: undefined,
			afterIndex: added.afterIndex,
			insertionStrategy: insertPoint.strategy
		})
	}

	// Finalize positions
	const items = timelineItems.map((item, index) => ({ ...item, position: index }))

	// Build merged flow
	const mergedModules = items.map((item) => item.module)
	const isRemoved = (id: string | undefined) =>
		id !== undefined &&
		(moduleDiff[id]?.before === 'removed' || moduleDiff[id]?.before === 'shadowed')

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

	// Build itemMap
	const itemMap = new Map<string, TimelineItem>()

	for (const item of items) {
		itemMap.set(item.module.id, item)
	}

	// Calculate statistics
	const stats = {
		added: items.filter((i) => i.operation === 'added').length,
		removed: items.filter((i) => i.operation === 'removed').length,
		shadowed: items.filter((i) => i.operation === 'shadowed').length,
		modified: items.filter((i) => i.operation === 'modified').length,
		kept: items.filter((i) => i.operation === undefined).length
	}

	return { items, itemMap, stats, mergedFlow }
}

/**
 * Finds insertion point for a new module by looking at its neighbors in afterFlow.
 */
function findInsertionPoint(
	addedModule: { id: string; afterIndex?: number; module: FlowModule },
	timelineItems: TimelineItem[],
	afterFlowModules: FlowModule[]
): { index: number; strategy: 'original_position' | 'anchor_based' | 'end_of_flow' } {
	if (addedModule.afterIndex === undefined) {
		return { index: timelineItems.length, strategy: 'end_of_flow' }
	}

	// Find nearest anchor module (from beforeFlow) before and after this position
	let prevAnchor: string | undefined
	for (let i = addedModule.afterIndex - 1; i >= 0; i--) {
		const id = afterFlowModules[i]?.id
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
	for (let i = addedModule.afterIndex + 1; i < afterFlowModules.length; i++) {
		const id = afterFlowModules[i]?.id
		if (
			timelineItems.find(
				(item) => item.module.id === id && item.insertionStrategy === 'original_position'
			)
		) {
			nextAnchor = id
			break
		}
	}

	// Insert between anchors
	if (prevAnchor) {
		const prevPos = timelineItems.findIndex((item) => item.module.id === prevAnchor)
		return { index: prevPos + 1, strategy: 'anchor_based' }
	} else if (nextAnchor) {
		const nextPos = timelineItems.findIndex((item) => item.module.id === nextAnchor)
		return { index: nextPos, strategy: 'anchor_based' }
	} else {
		return { index: timelineItems.length, strategy: 'end_of_flow' }
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
