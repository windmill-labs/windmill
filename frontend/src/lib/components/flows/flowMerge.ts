import type { FlowModule, FlowValue } from '$lib/gen'
import type { ModuleDiffResult } from './flowDiff'

/**
 * Merges two flow versions into a single flow structure that includes all modules from both.
 * This is used for the unified diff view where we want to show added, modified, and removed modules.
 *
 * Strategy:
 * - Use the after flow structure as the base
 * - Inject removed modules from the before flow at the end of their respective arrays
 * - Recursively merge nested structures (loops, branches, AI agent tools)
 *
 * @param beforeFlow - The original flow
 * @param afterFlow - The modified flow
 * @param moduleDiff - The computed diff result with before/after actions per module
 * @returns A merged flow containing all modules from both flows
 */
export function mergeFlows(
	beforeFlow: FlowValue,
	afterFlow: FlowValue,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowValue {
	// Start with after flow as base and merge modules recursively
	const mergedModules = mergeModuleArrays(
		beforeFlow.modules ?? [],
		afterFlow.modules ?? [],
		moduleDiff
	)

	// Merge special modules (failure and preprocessor)
	const mergedFailureModule = mergeSingleModule(
		beforeFlow.failure_module,
		afterFlow.failure_module,
		moduleDiff
	)

	const mergedPreprocessorModule = mergeSingleModule(
		beforeFlow.preprocessor_module,
		afterFlow.preprocessor_module,
		moduleDiff
	)

	// Return merged flow with after flow's top-level properties
	return {
		...afterFlow,
		modules: mergedModules,
		failure_module: mergedFailureModule,
		preprocessor_module: mergedPreprocessorModule
	}
}

/**
 * Merges a single optional module (failure_module or preprocessor_module)
 */
function mergeSingleModule(
	beforeModule: FlowModule | undefined,
	afterModule: FlowModule | undefined,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule | undefined {
	// If only in after or exists in both, use after version
	if (afterModule) {
		return mergeModuleNested(beforeModule, afterModule, moduleDiff)
	}

	// If only in before (removed), include it so it shows with red color
	if (beforeModule) {
		return mergeModuleNested(beforeModule, undefined, moduleDiff)
	}

	return undefined
}

/**
 * Merges two module arrays, combining modules from both before and after.
 * Modules are identified by their ID.
 *
 * Logic:
 * - Modules in after: included (may be added or modified)
 * - Modules only in before: appended at end (removed)
 * - Nested structures: recursively merged
 */
function mergeModuleArrays(
	beforeModules: FlowModule[],
	afterModules: FlowModule[],
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule[] {
	const result: FlowModule[] = []
	const processedIds = new Set<string>()

	// First, include all modules from after (these are either added or modified or unchanged)
	for (const afterModule of afterModules) {
		const beforeModule = beforeModules.find((m) => m.id === afterModule.id)
		// Recursively merge nested structures
		const merged = mergeModuleNested(beforeModule, afterModule, moduleDiff)
		result.push(merged)
		processedIds.add(afterModule.id)
	}

	// Then, append modules that only exist in before (removed modules)
	for (const beforeModule of beforeModules) {
		if (!processedIds.has(beforeModule.id)) {
			// This module was removed, include it so it shows with red color
			const merged = mergeModuleNested(beforeModule, undefined, moduleDiff)
			result.push(merged)
		}
	}

	return result
}

/**
 * Merges a single module, handling nested structures recursively.
 * If the module contains nested modules (loops, branches, AI tools), merge those too.
 */
function mergeModuleNested(
	beforeModule: FlowModule | undefined,
	afterModule: FlowModule | undefined,
	moduleDiff: Record<string, ModuleDiffResult>
): FlowModule {
	// Use afterModule as base if it exists, otherwise use beforeModule
	const baseModule = afterModule ?? beforeModule!

	// Clone the module to avoid mutations
	const merged: FlowModule = JSON.parse(JSON.stringify(baseModule))

	// Handle nested structures based on module type
	const beforeValue = beforeModule?.value
	const afterValue = afterModule?.value

	if (merged.value.type === 'forloopflow' || merged.value.type === 'whileloopflow') {
		// Merge nested modules in loop
		const beforeNested = beforeValue?.type === merged.value.type ? beforeValue.modules : []
		const afterNested = afterValue?.type === merged.value.type ? afterValue.modules : []
		merged.value.modules = mergeModuleArrays(beforeNested, afterNested, moduleDiff)
	} else if (merged.value.type === 'branchone') {
		// Merge branches and default
		if (beforeValue?.type === 'branchone' && afterValue?.type === 'branchone') {
			// Merge default branch
			merged.value.default = mergeModuleArrays(
				beforeValue.default,
				afterValue.default,
				moduleDiff
			)

			// Merge each conditional branch (match by index)
			merged.value.branches = afterValue.branches.map((afterBranch, idx) => {
				const beforeBranch = beforeValue.branches[idx]
				return {
					...afterBranch,
					modules: mergeModuleArrays(
						beforeBranch?.modules ?? [],
						afterBranch.modules,
						moduleDiff
					)
				}
			})

			// If before had more branches than after, append them (rare but possible)
			for (let i = afterValue.branches.length; i < beforeValue.branches.length; i++) {
				const removedBranch = beforeValue.branches[i]
				merged.value.branches.push({
					...removedBranch,
					modules: mergeModuleArrays(removedBranch.modules, [], moduleDiff)
				})
			}
		} else if (afterValue?.type === 'branchone') {
			// After is branchone but before is not (or doesn't exist) - use after as-is
			// Already handled by baseModule
		} else if (beforeValue?.type === 'branchone') {
			// Before is branchone but after is not - use before structure with all modules marked as removed
			merged.value = beforeValue
			merged.value.default = mergeModuleArrays(beforeValue.default, [], moduleDiff)
			merged.value.branches = beforeValue.branches.map((branch) => ({
				...branch,
				modules: mergeModuleArrays(branch.modules, [], moduleDiff)
			}))
		}
	} else if (merged.value.type === 'branchall') {
		// Merge all branches
		if (beforeValue?.type === 'branchall' && afterValue?.type === 'branchall') {
			merged.value.branches = afterValue.branches.map((afterBranch, idx) => {
				const beforeBranch = beforeValue.branches[idx]
				return {
					...afterBranch,
					modules: mergeModuleArrays(
						beforeBranch?.modules ?? [],
						afterBranch.modules,
						moduleDiff
					)
				}
			})

			// If before had more branches than after, append them
			for (let i = afterValue.branches.length; i < beforeValue.branches.length; i++) {
				const removedBranch = beforeValue.branches[i]
				merged.value.branches.push({
					...removedBranch,
					modules: mergeModuleArrays(removedBranch.modules, [], moduleDiff)
				})
			}
		} else if (afterValue?.type === 'branchall') {
			// After is branchall but before is not - use after as-is
		} else if (beforeValue?.type === 'branchall') {
			// Before is branchall but after is not - use before structure
			merged.value = beforeValue
			merged.value.branches = beforeValue.branches.map((branch) => ({
				...branch,
				modules: mergeModuleArrays(branch.modules, [], moduleDiff)
			}))
		}
	} else if (merged.value.type === 'aiagent') {
		// Merge AI agent tools
		if (beforeValue?.type === 'aiagent' && afterValue?.type === 'aiagent') {
			merged.value.tools = afterValue.tools.map((afterTool, idx) => {
				const beforeTool = beforeValue.tools[idx]
				// Tools can be flowmodules or mcp tools
				if (afterTool.value.tool_type === 'flowmodule' && beforeTool?.value.tool_type === 'flowmodule') {
					// Both are flowmodule tools - recursively merge if nested
					// For now, just use after tool structure
					// (This is complex and may need more sophisticated merging)
					return afterTool
				}
				return afterTool
			})

			// If before had more tools than after, append them
			for (let i = afterValue.tools.length; i < beforeValue.tools.length; i++) {
				merged.value.tools.push(beforeValue.tools[i])
			}
		}
	}

	return merged
}
