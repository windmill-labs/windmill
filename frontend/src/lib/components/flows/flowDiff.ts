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
 * Merges two flow versions into a single flow structure that includes all modules from both.
 * This is used for the unified diff view where we want to show added, modified, and removed modules.
 *
 * Strategy:
 * - Use the before flow structure as the base (removed modules already in their positions)
 * - Append added modules from the after flow at the end of their respective arrays
 * - Recursively handle nested structures (loops, branches, AI agent tools)
 *
 * @param beforeFlow - The original flow
 * @param afterFlow - The modified flow
 * @param moduleDiff - The computed diff result with before/after actions per module
 * @returns A merged flow containing all modules from both flows
 */
export function mergeFlowsSimple(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowValue {
	// Start with before flow as base (has all removed modules in position)
	const mergedModules = appendAddedModules(
		beforeFlow.modules ?? [],
		afterFlow.modules ?? [],
		moduleDiff
	)

	// Handle special modules
	const mergedFailureModule = appendSingleModule(
		beforeFlow.failure_module,
		afterFlow.failure_module,
		moduleDiff
	)

	const mergedPreprocessorModule = appendSingleModule(
		beforeFlow.preprocessor_module,
		afterFlow.preprocessor_module,
		moduleDiff
	)

	// Return merged flow with before flow's top-level properties but after flow's metadata
	return {
		...beforeFlow,
		...afterFlow,
		modules: mergedModules,
		failure_module: mergedFailureModule,
		preprocessor_module: mergedPreprocessorModule
	}
}

/**
 * Handles a single optional module (failure_module or preprocessor_module).
 * If module was added, include the after version.
 * Otherwise, keep the before version (which might be removed or modified).
 */
function appendSingleModule(
	beforeModule: FlowModule | undefined,
	afterModule: FlowModule | undefined,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule | undefined {
	// If the after module was added (not in before), return the after module with nested handling
	if (afterModule && moduleDiff[afterModule.id]?.after === 'added') {
		return appendNestedModules(undefined, afterModule, moduleDiff)
	}

	// Otherwise, keep the before module (handles removed and modified cases)
	if (beforeModule) {
		return appendNestedModules(beforeModule, afterModule, moduleDiff)
	}

	return undefined
}

/**
 * Appends added modules from afterModules to beforeModules array.
 * Modules are identified by their ID.
 *
 * Logic:
 * - Start with all modules from before (includes removed modules)
 * - Append modules from after that are marked as 'added'
 * - For modules in both, recursively handle nested structures
 */
function appendAddedModules(
	beforeModules: FlowModule[],
	afterModules: FlowModule[],
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule[] {
	const result: FlowModule[] = []

	// First, process all modules from before (includes removed and modified)
	for (const beforeModule of beforeModules) {
		const afterModule = afterModules.find((m) => m.id === beforeModule.id)
		// Recursively handle nested structures
		const processed = appendNestedModules(beforeModule, afterModule, moduleDiff)
		result.push(processed)
	}

	// Then, append modules that only exist in after (added modules)
	for (const afterModule of afterModules) {
		if (moduleDiff[afterModule.id]?.after === 'added') {
			const processed = appendNestedModules(undefined, afterModule, moduleDiff)
			result.push(processed)
		}
	}

	return result
}

/**
 * Handles a single module and its nested structures.
 * If the module contains nested modules (loops, branches, AI tools), recursively append added modules.
 */
function appendNestedModules(
	beforeModule: FlowModule | undefined,
	afterModule: FlowModule | undefined,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule {
	// Use beforeModule as base if it exists, otherwise use afterModule
	const baseModule = beforeModule ?? afterModule!

	// Clone the module to avoid mutations
	const processed: FlowModule = JSON.parse(JSON.stringify(baseModule))

	// Handle nested structures based on module type
	const beforeValue = beforeModule?.value
	const afterValue = afterModule?.value

	// Only process nested structures if types match (otherwise it's a different module)
	if (processed.value.type === 'forloopflow' || processed.value.type === 'whileloopflow') {
		// Handle nested modules in loop
		const beforeNested = beforeValue?.type === processed.value.type ? beforeValue.modules ?? [] : []
		const afterNested = afterValue?.type === processed.value.type ? afterValue.modules ?? [] : []
		processed.value.modules = appendAddedModules(beforeNested, afterNested, moduleDiff)
	} else if (processed.value.type === 'branchone') {
		// Handle branches and default
		if (beforeValue?.type === 'branchone' && afterValue?.type === 'branchone') {
			// Process default branch
			processed.value.default = appendAddedModules(
				beforeValue.default ?? [],
				afterValue.default ?? [],
				moduleDiff
			)

			// Process branches - use before branches as base, match with after by index
			processed.value.branches = beforeValue.branches.map((beforeBranch, idx) => {
				const afterBranch = afterValue.branches[idx]
				return {
					...beforeBranch,
					modules: appendAddedModules(
						beforeBranch.modules ?? [],
						afterBranch?.modules ?? [],
						moduleDiff
					)
				}
			})

			// Append any extra branches that were added in after
			for (let i = beforeValue.branches.length; i < afterValue.branches.length; i++) {
				const addedBranch = afterValue.branches[i]
				processed.value.branches.push({
					...addedBranch,
					modules: appendAddedModules([], addedBranch.modules ?? [], moduleDiff)
				})
			}
		} else if (beforeValue?.type === 'branchone') {
			// Only before has branchone - keep it with all its nested modules
			processed.value.default = appendAddedModules(beforeValue.default ?? [], [], moduleDiff)
			processed.value.branches = beforeValue.branches.map((branch) => ({
				...branch,
				modules: appendAddedModules(branch.modules ?? [], [], moduleDiff)
			}))
		} else if (afterValue?.type === 'branchone') {
			// Only after has branchone - it's all added
			processed.value = afterValue
			processed.value.default = appendAddedModules([], afterValue.default ?? [], moduleDiff)
			processed.value.branches = afterValue.branches.map((branch) => ({
				...branch,
				modules: appendAddedModules([], branch.modules ?? [], moduleDiff)
			}))
		}
	} else if (processed.value.type === 'branchall') {
		// Handle parallel branches
		if (beforeValue?.type === 'branchall' && afterValue?.type === 'branchall') {
			processed.value.branches = beforeValue.branches.map((beforeBranch, idx) => {
				const afterBranch = afterValue.branches[idx]
				return {
					...beforeBranch,
					modules: appendAddedModules(
						beforeBranch.modules ?? [],
						afterBranch?.modules ?? [],
						moduleDiff
					)
				}
			})

			// Append any extra branches added in after
			for (let i = beforeValue.branches.length; i < afterValue.branches.length; i++) {
				const addedBranch = afterValue.branches[i]
				processed.value.branches.push({
					...addedBranch,
					modules: appendAddedModules([], addedBranch.modules ?? [], moduleDiff)
				})
			}
		} else if (beforeValue?.type === 'branchall') {
			// Only before has branchall
			processed.value.branches = beforeValue.branches.map((branch) => ({
				...branch,
				modules: appendAddedModules(branch.modules ?? [], [], moduleDiff)
			}))
		} else if (afterValue?.type === 'branchall') {
			// Only after has branchall
			processed.value = afterValue
			processed.value.branches = afterValue.branches.map((branch) => ({
				...branch,
				modules: appendAddedModules([], branch.modules ?? [], moduleDiff)
			}))
		}
	}
	// Note: aiagent tools merging is complex and rarely changes, so we keep the simple approach
	// of using the base module's structure as-is

	return processed
}
