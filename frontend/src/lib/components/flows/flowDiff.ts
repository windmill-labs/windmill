import type { FlowModule, FlowValue } from '$lib/gen'
import { dfs } from './dfs'
import { deepEqual } from 'fast-equals'
import type { AIModuleAction } from '../copilot/chat/flow/core'

/**
 * Computes the difference between two flow versions and returns a map of module IDs to their actions.
 *
 * @param beforeFlow - The original flow value
 * @param afterFlow - The modified flow value
 * @returns A record mapping module IDs to their action type ('added', 'removed', or 'modified')
 */
export function computeFlowModuleDiff(
	beforeFlow: FlowValue,
	afterFlow: FlowValue
): Record<string, AIModuleAction> {
	const result: Record<string, AIModuleAction> = {}

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
			result[moduleId] = 'added'
		} else if (beforeModule && !afterModule) {
			// Module exists in before but not after -> removed
			result[moduleId] = 'removed'
		} else if (beforeModule && afterModule) {
			// Module exists in both -> check if modified
			if (!deepEqual(beforeModule, afterModule)) {
				result[moduleId] = 'modified'
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
 * - Before view: shows 'removed' and 'modified' modules
 * - After view: shows 'added' and 'modified' modules
 *
 * @param moduleDiff - The complete module diff
 * @returns An object with beforeActions and afterActions maps
 */
export function splitModuleDiffForViews(moduleDiff: Record<string, AIModuleAction>): {
	beforeActions: Record<string, AIModuleAction>
	afterActions: Record<string, AIModuleAction>
} {
	const beforeActions: Record<string, AIModuleAction> = {}
	const afterActions: Record<string, AIModuleAction> = {}

	for (const [moduleId, action] of Object.entries(moduleDiff)) {
		if (action === 'removed' || action === 'modified') {
			beforeActions[moduleId] = action
		}
		if (action === 'added' || action === 'modified') {
			afterActions[moduleId] = action
		}
	}

	return { beforeActions, afterActions }
}
