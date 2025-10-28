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
 * @returns A merged flow containing all top-level modules positioned appropriately
 */
export function mergeFlows(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowValue {
	// Get top-level removed modules from before flow with their original indices
	const removedModulesWithIndex = (beforeFlow.modules ?? [])
		.map((m, index) => ({ module: m, index }))
		.filter(({ module }) => moduleDiff[module.id]?.before === 'removed')

	// Create items for after modules with their current index
	const afterModulesWithIndex = (afterFlow.modules ?? []).map((m, index) => ({
		module: m,
		index,
		isFromAfter: true
	}))

	// Create items for removed modules with their original index
	const removedModulesItems = removedModulesWithIndex
		.filter(({ module }) => {
			const originalIndex = moduleDiff[module.id]?.originalIndex
			return originalIndex !== undefined
		})
		.map(({ module }) => ({
			module,
			index: moduleDiff[module.id].originalIndex!,
			isFromAfter: false
		}))

	// Merge and sort by position
	// Tie-breaker: removed modules (isFromAfter=false) come before after modules at the same index
	const allModules = [...afterModulesWithIndex, ...removedModulesItems].sort((a, b) => {
		if (a.index !== b.index) {
			return a.index - b.index
		}
		// At same index: removed modules first (show what was there before)
		return a.isFromAfter === b.isFromAfter ? 0 : a.isFromAfter ? 1 : -1
	})

	const mergedModules = allModules.map((item) => item.module)

	// Handle special modules - use after version or add if removed
	const mergedFailureModule =
		afterFlow.failure_module ??
		(beforeFlow.failure_module && moduleDiff[beforeFlow.failure_module.id]?.before === 'removed'
			? beforeFlow.failure_module
			: undefined)

	const mergedPreprocessorModule =
		afterFlow.preprocessor_module ??
		(beforeFlow.preprocessor_module &&
		moduleDiff[beforeFlow.preprocessor_module.id]?.before === 'removed'
			? beforeFlow.preprocessor_module
			: undefined)

	return {
		...afterFlow,
		modules: mergedModules,
		failure_module: mergedFailureModule,
		preprocessor_module: mergedPreprocessorModule
	}
}
