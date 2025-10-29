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
 * Merges two flow versions into a single flow structure for unified diff view.
 * This is a non-recursive approach that preserves the after flow structure
 * and inserts top-level removed modules at their original positions.
 *
 * Strategy:
 * - Use the after flow structure as-is (includes all added modules, even nested ones)
 * - Insert top-level removed modules at their original positions
 * - Don't merge nested structures - containers with nested changes will be marked as "modified" in the UI
 *
 * @param beforeFlow - The original flow
 * @param afterFlow - The modified flow
 * @param moduleDiff - The computed diff result with before/after actions per module
 * @param markRemovedAsShadowed - If true, mark removed modules as 'shadowed' instead of 'removed' (for after graph view)
 * @returns A merged flow containing all top-level modules positioned appropriately
 */
export function mergeFlows(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>,
	markRemovedAsShadowed: boolean = false
): {
	diff: Record<string, ModuleDiffResult>
	mergedFlow: FlowValue
} {
	let diff: Record<string, ModuleDiffResult> = { ...moduleDiff }

	// Get top-level removed modules from before flow with their original indices
	const removedModulesWithIndex = (beforeFlow.modules ?? [])
		.map((m, index) => ({ module: m, index }))
		.filter(({ module }) => moduleDiff[module.id]?.before === 'removed')

	// Create a set of ALL after module IDs (including nested) for conflict detection
	const afterModules = getAllModulesMap(afterFlow)
	const afterModuleIds = new Set(afterModules.keys())

	// Create items for after modules with their current index
	const afterModulesWithIndex = (afterFlow.modules ?? []).map((m, index) => ({
		module: m,
		index,
		isFromAfter: true
	}))

	// Create items for removed modules with their original index
	// Handle ID conflicts by prefixing with __removed__
	// Preserve array position to maintain correct ordering for consecutive removed modules
	const removedModulesItems = removedModulesWithIndex
		.filter(({ module }) => {
			const originalIndex = moduleDiff[module.id]?.originalIndex
			return originalIndex !== undefined
		})
		.map(({ module, index: arrayPosition }) => {
			const hasConflict = afterModuleIds.has(module.id)

			if (hasConflict) {
				const renamedId = `__removed__${module.id}`

				// Update diff map with renamed ID
				// Mark as 'shadowed' if requested, otherwise keep as 'removed'
				diff[renamedId] = {
					...diff[module.id],
					originalIndex: diff[module.id].originalIndex,
					before: markRemovedAsShadowed ? 'shadowed' : diff[module.id].before
				}

				return {
					module: { ...module, id: renamedId },
					index: diff[module.id].originalIndex!,
					arrayPosition,
					isFromAfter: false
				}
			}

			// Mark as 'shadowed' if requested, otherwise keep as 'removed'
			if (markRemovedAsShadowed && diff[module.id]) {
				diff[module.id] = {
					...diff[module.id],
					before: 'shadowed'
				}
			}

			return {
				module,
				index: diff[module.id].originalIndex!,
				arrayPosition,
				isFromAfter: false
			}
		})

	// Merge and sort by position
	// Tie-breaker: removed modules (isFromAfter=false) come before after modules at the same index
	// Secondary tie-breaker: for consecutive removed modules, use array position to maintain order
	const allModules = [...afterModulesWithIndex, ...removedModulesItems].sort((a, b) => {
		if (a.index !== b.index) {
			return a.index - b.index
		}
		// At same index: removed modules first (show what was there before)
		if (a.isFromAfter !== b.isFromAfter) {
			return a.isFromAfter ? 1 : -1
		}
		// Both are removed modules: use array position to maintain original order
		const aPos = (a as any).arrayPosition ?? 0
		const bPos = (b as any).arrayPosition ?? 0
		return aPos - bPos
	})

	const mergedModules = allModules.map((item) => item.module)

	// Handle special modules - use after version or add if removed
	// Also mark them as shadowed if requested
	let mergedFailureModule =
		afterFlow.failure_module ??
		(beforeFlow.failure_module && moduleDiff[beforeFlow.failure_module.id]?.before === 'removed'
			? beforeFlow.failure_module
			: undefined)

	let mergedPreprocessorModule =
		afterFlow.preprocessor_module ??
		(beforeFlow.preprocessor_module &&
		moduleDiff[beforeFlow.preprocessor_module.id]?.before === 'removed'
			? beforeFlow.preprocessor_module
			: undefined)

	// Mark special modules as shadowed if requested
	if (markRemovedAsShadowed) {
		if (mergedFailureModule && moduleDiff[mergedFailureModule.id]?.before === 'removed') {
			moduleDiff[mergedFailureModule.id] = {
				...moduleDiff[mergedFailureModule.id],
				before: 'shadowed'
			}
		}
		if (mergedPreprocessorModule && moduleDiff[mergedPreprocessorModule.id]?.before === 'removed') {
			moduleDiff[mergedPreprocessorModule.id] = {
				...moduleDiff[mergedPreprocessorModule.id],
				before: 'shadowed'
			}
		}
	}

	return {
		diff,
		mergedFlow: {
			...afterFlow,
			modules: mergedModules,
			failure_module: mergedFailureModule,
			preprocessor_module: mergedPreprocessorModule
		}
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
