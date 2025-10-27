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
			result[moduleId] = { before: 'removed' }
		} else if (beforeModule && afterModule) {
			// Module exists in both -> check type and content
			const typeChanged = beforeModule.value.type !== afterModule.value.type

			if (typeChanged) {
				// Type changed -> treat as removed + added
				result[moduleId] = { before: 'removed', after: 'added' }
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
 * This is a simple non-recursive approach that preserves the before flow structure
 * and appends top-level added modules.
 *
 * Strategy:
 * - Use the before flow structure as-is (keeps removed modules in their positions)
 * - Append top-level added modules at the end
 * - Don't merge nested structures - containers with nested changes will be marked as "modified" in the UI
 *
 * @param beforeFlow - The original flow
 * @param afterFlow - The modified flow
 * @param moduleDiff - The computed diff result with before/after actions per module
 * @returns A merged flow containing all top-level modules
 */
export function mergeFlows(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowValue {
	// Get top-level added modules from after flow
	const addedModules = (afterFlow.modules ?? []).filter((m) => moduleDiff[m.id]?.after === 'added')

	// Start with before modules and append added ones
	const mergedModules = [...(beforeFlow.modules ?? []), ...addedModules]

	// Handle special modules - use before version or add if new
	const mergedFailureModule =
		beforeFlow.failure_module ??
		(afterFlow.failure_module && moduleDiff[afterFlow.failure_module.id]?.after === 'added'
			? afterFlow.failure_module
			: undefined)

	const mergedPreprocessorModule =
		beforeFlow.preprocessor_module ??
		(afterFlow.preprocessor_module &&
		moduleDiff[afterFlow.preprocessor_module.id]?.after === 'added'
			? afterFlow.preprocessor_module
			: undefined)

	return {
		...beforeFlow,
		modules: mergedModules,
		failure_module: mergedFailureModule,
		preprocessor_module: mergedPreprocessorModule
	}
}
