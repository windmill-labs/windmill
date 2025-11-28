/**
 * Flow Diff Manager
 *
 * A reusable store for managing flow diff state, module actions, and accept/reject operations.
 * This decouples diff management from specific UI components (like AI chat) and makes it
 * available for any use case that needs to track and apply flow changes.
 */

import type { ExtendedOpenFlow } from './types'
import type { FlowModule, FlowValue } from '$lib/gen'
import type { ModuleActionInfo } from '../copilot/chat/flow/core'
import {
	buildFlowTimeline,
	insertModuleIntoFlow,
	findModuleParent,
	DUPLICATE_MODULE_PREFIX
} from './flowDiff'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import { getIndexInNestedModules } from '../copilot/chat/flow/utils'
import { dfs } from './previousResults'
import type DiffDrawer from '../DiffDrawer.svelte'

export type FlowDiffManager = ReturnType<typeof createFlowDiffManager>

/**
 * Options for computing diff
 */
export type ComputeDiffOptions = {
	/** Mark all changes as pending (requiring user approval) */
	markAsPending?: boolean
	/** Mark removed modules as shadowed instead of removed (for visualization) */
	markRemovedAsShadowed?: boolean
}

/**
 * Creates a flow diff manager instance
 */
function createSkeletonModule(module: FlowModule): FlowModule {
	const clone = JSON.parse(JSON.stringify(module))
	if (clone.value.type === 'forloopflow' || clone.value.type === 'whileloopflow') {
		clone.value.modules = []
	} else if (clone.value.type === 'branchone') {
		clone.value.default = []
		clone.value.branches.forEach((b: any) => (b.modules = []))
	} else if (clone.value.type === 'branchall') {
		clone.value.branches.forEach((b: any) => (b.modules = []))
	} else if (clone.value.type === 'aiagent') {
		clone.value.tools = []
	}
	return clone
}

export function createFlowDiffManager() {
	// State: snapshot of flow before changes
	let beforeFlow = $state<ExtendedOpenFlow | undefined>(undefined)

	// State: current flow after changes
	let afterFlow = $state<FlowValue | undefined>(undefined)

	// State: merged flow containing both original and modified/removed modules
	let mergedFlow = $state<FlowValue | undefined>(undefined)

	// State: input schema after changes (beforeInputSchema is just beforeFlow?.schema)
	let afterInputSchema = $state<Record<string, any> | undefined>(undefined)

	// State: whether to mark removed modules as shadowed (for side-by-side view)
	let markRemovedAsShadowed = $state(false)

	// State: whether to allow accepting/rejecting changes to the flow
	let editMode = $state(false)

	// State: module actions tracking changes (added/modified/removed/shadowed)
	let moduleActions = $state<Record<string, ModuleActionInfo>>({})

	// Reference to DiffDrawer component for showing module diffs (not reactive)
	let diffDrawer: DiffDrawer | undefined = undefined

	// Derived: whether there are any pending changes
	const hasPendingChanges = $derived(Object.values(moduleActions).some((info) => info.pending))

	// Auto-compute diff when beforeFlow or afterFlow changes
	$effect(() => {
		if (beforeFlow && afterFlow) {
			const timeline = buildFlowTimeline(beforeFlow.value, afterFlow, {
				markRemovedAsShadowed: markRemovedAsShadowed,
				markAsPending: editMode
			})

			// Store the merged flow for rendering
			mergedFlow = timeline.mergedFlow

			// Update module actions
			const newActions = { ...timeline.afterActions }

			// Check for input schema changes
			if (beforeFlow.schema && afterInputSchema) {
				const schemaChanged = JSON.stringify(beforeFlow.schema) !== JSON.stringify(afterInputSchema)
				if (schemaChanged) {
					newActions['Input'] = {
						action: 'modified',
						pending: editMode
					}
				}
			}

			updateModuleActions(newActions)

			// If no more actions, clear the snapshot (exit diff mode)
			if (Object.keys(newActions).length === 0) {
				clearSnapshot()
			}
		} else if (!beforeFlow) {
			// Clear module actions and merged flow when no snapshot
			mergedFlow = undefined
			updateModuleActions({})
		}
	})

	/**
	 * Helper to update moduleActions and notify listeners
	 */
	function updateModuleActions(newActions: Record<string, ModuleActionInfo>) {
		moduleActions = newActions
	}

	/**
	 * Set the before flow snapshot for diff computation
	 */
	function setSnapshot(flow: ExtendedOpenFlow | undefined) {
		beforeFlow = flow
	}

	/**
	 * Set the after flow (current state) for diff computation
	 */
	function setAfterFlow(flow: FlowValue | undefined) {
		afterFlow = flow
	}

	/**
	 * Set the after input schema for tracking schema changes
	 */
	function setAfterInputSchema(schema: Record<string, any> | undefined) {
		afterInputSchema = schema
	}

	/**
	 * Set whether to mark removed modules as shadowed (for side-by-side view)
	 */
	function setMarkRemovedAsShadowed(value: boolean) {
		markRemovedAsShadowed = value
	}

	/**
	 * Set whether to edit the flow
	 */
	function setEditMode(value: boolean) {
		editMode = value
	}

	/**
	 * Clear the snapshot and all module actions
	 */
	function clearSnapshot() {
		beforeFlow = undefined
		afterFlow = undefined
		mergedFlow = undefined
		afterInputSchema = undefined
		updateModuleActions({})
	}

	/**
	 * Get the current before flow snapshot
	 */
	function getSnapshot(): ExtendedOpenFlow | undefined {
		return beforeFlow
	}

	/**
	 * Set module actions directly (useful when actions are computed elsewhere)
	 */
	function setModuleActions(actions: Record<string, ModuleActionInfo>) {
		updateModuleActions(actions)
	}

	/**
	 * Get current module actions
	 */
	function getModuleActions(): Record<string, ModuleActionInfo> {
		return moduleActions
	}

	/**
	 * Helper to get a module from a flow by ID
	 */
	function getModuleFromFlow(id: string, flow: ExtendedOpenFlow): FlowModule | undefined {
		if (flow.value.preprocessor_module?.id === id) {
			return flow.value.preprocessor_module
		} else if (flow.value.failure_module?.id === id) {
			return flow.value.failure_module
		} else {
			return dfs(id, flow, false)[0]
		}
	}

	/**
	 * Internal helper to delete a module from a flow object
	 */
	function deleteModuleInternal(id: string, flow: ExtendedOpenFlow) {
		if (flow.value.preprocessor_module?.id === id) {
			flow.value.preprocessor_module = undefined
		} else if (flow.value.failure_module?.id === id) {
			flow.value.failure_module = undefined
		} else {
			const { modules } = getIndexInNestedModules(flow, id)
			const index = modules.findIndex((m) => m.id === id)
			if (index >= 0) {
				modules.splice(index, 1)
			}
		}
	}

	/**
	 * Helper to delete a module from the flow
	 */
	function deleteModuleFromFlow(
		id: string,
		flowStore: StateStore<ExtendedOpenFlow>,
		selectNextIdFn?: (id: string) => void
	) {
		selectNextIdFn?.(id)
		deleteModuleInternal(id, flowStore.val)
		refreshStateStore(flowStore)
	}

	/**
	 * Accept a module action (keep the changes)
	 * Removes the action from tracking after acceptance
	 */
	function acceptModule(id: string, flowStore?: StateStore<ExtendedOpenFlow>, asSkeleton = false) {
		if (!beforeFlow || !afterFlow) {
			throw new Error('Cannot accept module without beforeFlow and afterFlow snapshots')
		}

		const info = moduleActions[id]
		if (!info) return

		const actualId = id.startsWith(DUPLICATE_MODULE_PREFIX)
			? id.substring(DUPLICATE_MODULE_PREFIX.length)
			: id

		if (id === 'Input') {
			// Accept input schema changes: update beforeFlow to match afterInputSchema
			if (beforeFlow.schema && afterInputSchema) {
				beforeFlow.schema = JSON.parse(JSON.stringify(afterInputSchema))
			}
		} else if (info.action === 'removed') {
			// Removed in after: Remove from beforeFlow
			deleteModuleInternal(actualId, beforeFlow)
		} else if (info.action === 'added') {
			// Added in after: Add to beforeFlow

			// Check if parent exists in beforeFlow; if not, recursively accept parent first.
			const parentLoc = findModuleParent(afterFlow, actualId)
			if (
				parentLoc &&
				parentLoc.type !== 'root' &&
				parentLoc.type !== 'failure' &&
				parentLoc.type !== 'preprocessor'
			) {
				const parentInBefore = getModuleFromFlow(parentLoc.parentId, beforeFlow)
				if (!parentInBefore) {
					// Parent is missing in beforeFlow. It must be pending acceptance.
					// Accept as skeleton to avoid auto-accepting all siblings.
					acceptModule(parentLoc.parentId, flowStore, true)
				}
			}

			// Use insertModuleIntoFlow targeting beforeFlow, sourcing position from afterFlow
			let module = getModuleFromFlow(actualId, {
				value: afterFlow,
				summary: ''
			} as ExtendedOpenFlow)

			if (module) {
				// Check if module already exists in beforeFlow (could be a skeleton from earlier acceptance)
				const existingModule = getModuleFromFlow(actualId, beforeFlow)

				if (existingModule) {
					// Module already exists (as skeleton or partial), update it in-place
					const moduleToApply = asSkeleton ? createSkeletonModule(module) : module
					Object.keys(existingModule).forEach((k) => delete (existingModule as any)[k])
					Object.assign(existingModule, $state.snapshot(moduleToApply))
				} else {
					// Module doesn't exist, insert it
					const moduleToInsert = asSkeleton ? createSkeletonModule(module) : module
					insertModuleIntoFlow(beforeFlow.value, $state.snapshot(moduleToInsert), afterFlow, actualId)
				}
			}
		} else if (info.action === 'modified') {
			// Modified: Apply modifications to beforeFlow module
			const beforeModule = getModuleFromFlow(actualId, beforeFlow)
			const afterModule = getModuleFromFlow(actualId, {
				value: afterFlow,
				summary: ''
			} as ExtendedOpenFlow)

			if (beforeModule && afterModule) {
				Object.keys(beforeModule).forEach((k) => delete (beforeModule as any)[k])
				Object.assign(beforeModule, $state.snapshot(afterModule))
			}
		}

		// Note: The $effect will automatically recompute the diff, clearing the action
		// since beforeFlow now matches afterFlow for this module.
	}

	/**
	 * Reject a module action (revert the changes)
	 * Removes the action from tracking after rejection
	 */
	function rejectModule(id: string, flowStore?: StateStore<ExtendedOpenFlow>) {
		if (!beforeFlow) {
			throw new Error('Cannot reject module without a beforeFlow snapshot')
		}

		const actualId = id.startsWith(DUPLICATE_MODULE_PREFIX)
			? id.substring(DUPLICATE_MODULE_PREFIX.length)
			: id
		const info = moduleActions[id]

		if (!info) return

		// Only perform revert operations if flowStore is provided
		if (flowStore) {
			if (id === 'Input') {
				// Revert input schema changes
				flowStore.val.schema = beforeFlow.schema
				afterInputSchema = flowStore.val.schema
			} else if (info.action === 'added') {
				// Added in after: Remove from flowStore (afterFlow)
				deleteModuleFromFlow(actualId, flowStore)
			} else if (info.action === 'removed') {
				// Removed in after: Restore to flowStore (afterFlow)
				// Source from beforeFlow
				const oldModule = getModuleFromFlow(actualId, beforeFlow)
				if (oldModule) {
					insertModuleIntoFlow(
						flowStore.val.value,
						$state.snapshot(oldModule),
						beforeFlow.value,
						actualId
					)
				}
				refreshStateStore(flowStore)
			} else if (info.action === 'modified') {
				// Modified: Revert modifications in flowStore (afterFlow)
				const oldModule = getModuleFromFlow(actualId, beforeFlow)
				const newModule = getModuleFromFlow(actualId, flowStore.val)

				if (oldModule && newModule) {
					Object.keys(newModule).forEach((k) => delete (newModule as any)[k])
					Object.assign(newModule, $state.snapshot(oldModule))
				}
				refreshStateStore(flowStore)
			}

			afterFlow = flowStore.val.value
		}

		// Note: The $effect will automatically recompute the diff, clearing the action
		// since flowStore (afterFlow) now matches beforeFlow for this module.
	}

	/**
	 * Accept all pending module actions
	 */
	function acceptAll(flowStore?: StateStore<ExtendedOpenFlow>) {
		const ids = Object.keys(moduleActions)
		for (const id of ids) {
			if (moduleActions[id]?.pending) {
				acceptModule(id, flowStore)
			}
		}
	}

	/**
	 * Reject all pending module actions (in reverse order for nested modules)
	 */
	function rejectAll(flowStore?: StateStore<ExtendedOpenFlow>) {
		const ids = Object.keys(moduleActions)
		// Process in reverse to handle nested modules correctly
		for (let i = ids.length - 1; i >= 0; i--) {
			if (moduleActions[ids[i]]?.pending) {
				rejectModule(ids[i], flowStore)
			}
		}
	}

	/**
	 * Revert the entire flow to the snapshot
	 * @param flowStore - The flow store to update
	 * @param snapshot - Optional specific snapshot to revert to (defaults to beforeFlow)
	 */
	function revertToSnapshot(flowStore: StateStore<ExtendedOpenFlow>, snapshot?: ExtendedOpenFlow) {
		const targetSnapshot = snapshot ?? beforeFlow
		if (!targetSnapshot) return

		flowStore.val = targetSnapshot
		refreshStateStore(flowStore)
		clearSnapshot()
	}

	/**
	 * Set the DiffDrawer instance for showing module diffs
	 */
	function setDiffDrawer(drawer: DiffDrawer | undefined) {
		diffDrawer = drawer
	}

	/**
	 * Show diff for a specific module or Input schema
	 */
	function showModuleDiff(moduleId: string) {
		if (!diffDrawer || !beforeFlow) return

		if (moduleId === 'Input') {
			// Show input schema diff
			diffDrawer.openDrawer()
			diffDrawer.setDiff({
				mode: 'simple',
				title: 'Flow Input Schema Diff',
				original: { schema: beforeFlow.schema ?? {} },
				current: { schema: afterInputSchema ?? {} }
			})
		} else {
			// Show module diff
			const beforeModule = getModuleFromFlow(moduleId, beforeFlow)
			// Need to check failure_module and preprocessor_module for afterFlow as well
			let afterModule: FlowModule | undefined = undefined
			if (afterFlow) {
				if (afterFlow.preprocessor_module?.id === moduleId) {
					afterModule = afterFlow.preprocessor_module
				} else if (afterFlow.failure_module?.id === moduleId) {
					afterModule = afterFlow.failure_module
				} else {
					afterModule = dfs(moduleId, { value: afterFlow, summary: '' }, false)[0]
				}
			}

			if (beforeModule && afterModule) {
				diffDrawer.openDrawer()
				diffDrawer.setDiff({
					mode: 'simple',
					title: `Module Diff: ${moduleId}`,
					original: beforeModule,
					current: afterModule
				})
			}
		}
	}

	return {
		// State accessors
		get beforeFlow() {
			return beforeFlow
		},
		get afterFlow() {
			return afterFlow
		},
		get mergedFlow() {
			return mergedFlow
		},
		get moduleActions() {
			return moduleActions
		},
		get hasPendingChanges() {
			return hasPendingChanges
		},
		get afterInputSchema() {
			return afterInputSchema
		},
		get editModeEnabled() {
			return editMode
		},

		// Snapshot management
		setSnapshot,
		setAfterFlow,
		setAfterInputSchema,
		setMarkRemovedAsShadowed,
		setEditMode,
		clearSnapshot,
		getSnapshot,

		// Module actions management
		setModuleActions,
		getModuleActions,

		// Accept/reject operations
		acceptModule,
		rejectModule,
		acceptAll,
		rejectAll,
		revertToSnapshot,

		// Diff drawer management
		setDiffDrawer,
		showModuleDiff
	}
}
